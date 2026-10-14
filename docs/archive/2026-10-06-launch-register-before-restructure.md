# OpenFrame Studio — Master Product, Commercial, Security & Lifetime Operations Readiness Checklist — REVISED

> **Repository integration (2026-10-05).** Promoted from the supplied revised master checklist; this is the canonical
> register of **operations, legal, commerce, security, support and release** requirements. Product feature state lives in the
> [feature ledger](../implementation/01-feature-ledger.md); execution order and the Store gate live in the
> [masterplan](../implementation/00-masterplan.md). Rows here are referenced from masterplan items by ID.
> Corrections: the baseline commit is `master cd60f08` (same tree as `main d30b588`); commerce rows (`COMM-*`, `EXE-09-*`) apply only
> if the owner chooses a paid model (ADR-0018 / D1); analytics and crash-aggregation rows are reconciled by ADR-0021
> (no automatic telemetry); macOS rows (`MAC-*`, `EXE-07-*`) belong to milestone M16 and are not Store-gate items (ADR-0017).
> Additions from the intake notes are in Appendix F.

**Purpose:** the single execution control document for taking OpenFrame Studio from codebase to a commercially launchable, secure, supportable and maintainable desktop product under the umbrella company.

**Revision objective:** preserve exhaustive coverage while eliminating boilerplate repetition. Each requirement appears once in a canonical register; reusable gates are defined once and referenced by ID; operational runbooks and final audits do not restate every requirement.

**Important:** a checkbox is never evidence. An item is complete only when its required state and objective evidence exist.

**Baseline:** `Saravankumar25/OpenFrame` / `main` / `d30b5889ab5edf9f69d60f8e6d600cb11d4921a0`. The supplied completion plan recommends a dependency-aware strategy that preserves the durable core and reconciles old/new requirements by module. L1-L2

---

# 0. Status discipline

| **S0** | SPECIFIED | Requirement and source known. |
| **S1** | CONTRACTED | Data/IPC/UI/security/ownership contract defined. |
| **S2** | IMPLEMENTED | Real code exists and mechanical checks pass. |
| **S3** | REACHABLE | Normal user can discover/use it. |
| **S4** | VERIFIED | Lifecycle, persistence, failure, permissions, platform and interoperability evidence exists. |
| **S5** | RELEASE-READY | Release/security/support/commercial/documentation gates are green. |
| **S6** | OPERATED | Post-launch monitoring/support/backup/maintenance has been exercised. |
| **S7** | CONTINUITY-READY | A backup operator can recover/release/support the product. |

Allowed result labels: **OPEN / IN PROGRESS / BLOCKED / EVIDENCE PENDING / N/A / PASS / RISK ACCEPTED**.

N/A requires a reason. Critical N/A decisions require explicit approval. Risk acceptance never substitutes for unresolved customer-data loss, critical security or release-integrity failures.

The completion plan explicitly defines S0→S5 and the feature lifecycle Discover → Use → Save → Undo → Recover/Reopen → Export/Re-import/Continue. L1-L5

# 1. Current-state freeze

- [ ] Freeze the exact repository commit/version used as the release-program baseline.
- [ ] Record the current release declaration separately from implementation claims.
- [ ] Create the canonical feature ledger and evidence index.
- [ ] Create representative sample projects and import/export fixtures.
- [ ] Create AI retrieval/evaluation corpus and hardware matrix.
- [ ] Create Windows/macOS clean-machine matrix.
- [ ] Create commercial/store/account asset inventory.
- [ ] Create release evidence storage with immutable release identifiers.
- [ ] Keep unfinished capabilities out of launch claims until they reach S5.

The current evidence says the creative/production release declaration still marks major workspaces as in development/not usable, while Offline AI/assistant are implemented with retrieval-quality and hardware validation still pending. L1-L2

# 2. Non-negotiable architecture invariants

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

The supplied vibe-coding guidance identifies semantic drift as the primary risk and calls for architecture checks, lifecycle tests, migration fixtures, import/export corpora, AI evaluation and release gates. L1-L3

# 3. Cross-cutting quality gates

## G01 — Architecture
**Acceptance:** Registry allow-list; Store::mutate; UI cannot directly own canonical state; generated bindings and dependency rules.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G02 — Feature lifecycle
**Acceptance:** Discover → Use → Save → Undo → Recover/Reopen → Export/Package → Re-import/Reopen → Continue where applicable.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G03 — Persistence/migration
**Acceptance:** Fresh, legacy, representative, large and interrupted fixtures; integrity checks; safe failure.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G04 — Cross-platform
**Acceptance:** Windows x64 + macOS arm64 + macOS x64 when supported; same product/security semantics.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G05 — Round-trip
**Acceptance:** Import preview; canonical export; malformed-input safety; semantic round trip; no silent loss.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G06 — Security
**Acceptance:** Threat model; IPC; path confinement; parser bounds; secret hygiene; adversarial tests.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G07 — Privacy
**Acceptance:** Data inventory; minimization; disclosures; redaction; external-service review.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G08 — AI authority
**Acceptance:** Metadata; bounded tools; permission parity; human Change Sets; no direct canonical writes.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G09 — AI evaluation
**Acceptance:** Fixed corpus; retrieval metrics; adversarial tests; runtime regression; hardware validation.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G10 — Release artifact
**Acceptance:** Immutable version; checksum; signature; SBOM; license inventory; independent verification.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G11 — Installer/update
**Acceptance:** Clean install; upgrade; uninstall/reinstall; authenticity; update failure recovery; channel behavior.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G12 — Accessibility/UX
**Acceptance:** Keyboard/focus; reader semantics where applicable; DPI; IME; empty/loading/error/success states.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G13 — Performance
**Acceptance:** Defined startup, open, search, large-project, export and AI budgets.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G14 — Diagnostics
**Acceptance:** Safe logs; crash/error identity; diagnostic bundle; crash-loop recovery.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G15 — Documentation
**Acceptance:** Manual; install; recovery; import/export; AI; troubleshooting; policies; release notes.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G16 — Commercial consistency
**Acceptance:** Store ↔ website ↔ app ↔ entitlement ↔ email agree on price, trial, limits and refunds.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G17 — Supportability
**Acceptance:** Self-service; reproducible support; escalation; non-founder dependency.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G18 — Continuity
**Acceptance:** Backups; second admin; recovery drills; asset register; account-loss runbooks.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G19 — Marketing truth
**Acceptance:** Every public claim supported by shipped behavior and evidence.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

## G20 — Launch decision
**Acceptance:** P0 stop-ship items closed; exceptions formally accepted; evidence index complete; GO/NO-GO signed.

**Evidence:** link the automated result, fixture, artifact, review record or production drill in the evidence index.

---

# 4. Canonical requirement register

Each requirement appears once. Do not create duplicate copies in feature documents. Use gate IDs for shared acceptance rules.

| ID | Requirement | Priority | Risk | State | Gates | Evidence |
|---|---|---|---|---|---|---|
| `EXE-01-001` | React never owns canonical project truth | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-002` | React never opens SQLite directly | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-003` | React never writes arbitrary project files | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-004` | All canonical mutations pass through the approved Rust mutation pipeline | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-005` | Store::mutate or approved equivalent remains authoritative | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-006` | Registry is a strict allow-list | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-007` | Search is derived/rebuildable | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-008` | AI indexes are derived/rebuildable | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-009` | AI cannot directly mutate canonical state | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-010` | AI proposals require human Change Set approval | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-011` | AI cannot recursively/nestedly grant itself mutation authority | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-012` | Platform differences are hidden behind adapters | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-013` | No ADR-0007 LAN collaboration reintroduction | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-014` | Generated bindings are reproducible | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-01-015` | Forbidden dependency directions are mechanically checked | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `EXE-02-001` | Every schema change has a numbered migration | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-002` | Migration is transactional | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-003` | Migration has fresh-database fixture | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-004` | Migration has representative legacy fixture | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-005` | Migration has largest realistic fixture | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-006` | Migration has interrupted-process fixture | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-007` | Migration has disk-full fixture | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-008` | Migration has permission-denied fixture | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-009` | Migration has post-migration integrity_check | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-010` | Migration has search-index rebuild test | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-011` | Migration has AI-index rebuild test | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-012` | Migration preserves undo/redo semantics | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-013` | Migration preserves exportability | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-02-014` | Migration has documented downgrade policy | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `EXE-03-001` | Project format has explicit version | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-002` | Project has manifest | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-003` | Project records schema version | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-004` | Project records application compatibility | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-005` | Project package validates before import | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-006` | ZIP extraction rejects traversal | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-007` | Symlink escape is rejected | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-008` | Unicode paths are supported | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-009` | Long paths are tested | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-010` | Missing assets have deterministic behavior | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-011` | Windows-to-macOS round trip passes | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-012` | macOS-to-Windows round trip passes | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-013` | Customer can export important creative work | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-014` | Customer can recover after reinstall | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-03-015` | Customer can recover after machine replacement | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `EXE-04-001` | Element model is structured | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-002` | Element progression is deterministic | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-003` | Pagination is deterministic | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-004` | Dialogue continuation is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-005` | Parenthetical behavior is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-006` | Scene numbering is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-007` | Revision colors are tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-008` | Locked production draft is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-009` | Title page is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-010` | Find/replace is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-011` | Unicode is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-012` | Multilingual content is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-013` | FDX import preview is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-014` | FDX export is re-importable | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-015` | Unknown constructs are preserved/mapped/warned | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-016` | Fountain round trip is tested | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-017` | DOCX compatibility is documented | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-018` | PDF import is explicitly best-effort | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-019` | PDF export visual regression is automated | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-04-020` | Screenplay exports remain unbranded | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `EXE-05-001` | Malformed ZIP cannot escape destination | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-002` | Malformed XML cannot crash process unsafely | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-003` | Malformed PDF is safely rejected or degraded | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-004` | Malformed DOCX is safely rejected or degraded | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-005` | Oversized files are bounded | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-006` | Decompression bombs are bounded | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-007` | Import is transactional | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-008` | Import preview exposes warnings | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-009` | Import preserves original source | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-010` | Export uses canonical state | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-011` | Export failure cannot mutate canonical state | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-012` | Partial output is cleaned up | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-013` | Existing destination behavior is explicit | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-014` | Unicode destination paths work | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-05-015` | Permission failure is recoverable | P1 | Medium | S0 | G04, G05, G06, G20 | ☐ |
| `EXE-06-001` | Supported Windows versions are documented | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-002` | Clean install tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-003` | Upgrade tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-004` | Uninstall tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-005` | Reinstall tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-006` | WebView2 behavior verified | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-007` | AuthentiCode signature verified | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-008` | Timestamp verified | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-009` | SmartScreen behavior tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-010` | Windows Defender interaction tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-011` | High DPI tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-012` | Multiple monitors tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-013` | Unicode/IME tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-014` | Long paths tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-015` | External drive removal tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-016` | Read-only folder tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-017` | Network-share behavior explicitly defined | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-018` | Sleep/wake tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-019` | AI sidecar process cleanup tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-06-020` | Offline mode tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-001` | Supported macOS versions are documented | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-002` | Apple Silicon tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-003` | Intel tested if supported | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-004` | Developer ID signing verified | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-005` | Hardened Runtime enabled | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-006` | Entitlements minimized | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-007` | Secure timestamp verified | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-008` | Notarization verified | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-009` | Ticket stapled where applicable | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-010` | Gatekeeper tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-011` | Quarantine test performed | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-012` | DMG/PKG tested as applicable | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-013` | External drives tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-014` | File permissions tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-015` | Sleep/wake tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-016` | Unicode/case-sensitivity assumptions tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-017` | AI sidecar lifecycle tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-018` | Updater signing tested | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-019` | Clean-machine release test completed | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-07-020` | Developer account recovery documented | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `EXE-08-001` | Partner Center company ownership verified | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-002` | Backup administrator exists | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-003` | Publisher identity verified | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-004` | Package identity verified | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-005` | Store listing is complete | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-006` | Screenshots match build | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-007` | Description matches build | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-008` | Privacy URL works | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-009` | Support URL works | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-010` | Age rating completed | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-011` | Category completed | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-012` | Pricing verified | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-013` | Regional availability verified | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-014` | Trial configuration verified | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-015` | Certification notes prepared | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-016` | Windows App Certification Kit run where applicable | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-017` | Clean Store install tested | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-018` | Store update tested | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-019` | Store uninstall/reinstall tested | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-08-020` | Certification evidence archived | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `EXE-09-001` | 7-day trial starts correctly | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-002` | No credit card is required | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-003` | Trial expiry is deterministic | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-004` | Offline trial behavior is defined | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-005` | Reinstall behavior is defined | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-006` | Clock tampering behavior is defined | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-007` | Purchase during trial works | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-008` | Entitlement refresh works | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-009` | Restore purchase works | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-010` | Refund path is documented | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-011` | Cancellation path is documented | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-012` | Store outage behavior is defined | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-013` | Pricing is reconciled with finance | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-014` | Discount expiry has owner | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-09-015` | Commercial claims match actual flow | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-10-001` | Threat model exists | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-002` | IPC allow-list audited | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-003` | Filesystem scope audited | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-004` | Path traversal rejected | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-005` | Symlink escapes rejected | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-006` | Command injection tested | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-007` | SQL injection tested | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-008` | URL handling tested | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-009` | XML parser tested | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-010` | ZIP extraction tested | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-011` | PDF parser tested | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-012` | DOCX parser tested | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-013` | Fuzzing corpus exists | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-014` | Secret scan is clean | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-015` | Dependency scan reviewed | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-016` | License scan reviewed | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-017` | SBOM generated | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-018` | Signing keys protected | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-019` | Updater signatures verified | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-10-020` | Security disclosure channel tested | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `EXE-11-001` | Model license recorded | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-002` | Model checksum recorded | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-003` | Embedding license recorded | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-004` | Embedding checksum recorded | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-005` | Model download integrity verified | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-006` | Interrupted download tested | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-007` | Missing model handled | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-008` | Minimum hardware documented | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-009` | CPU fallback tested | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-010` | Vulkan path tested where supported | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-011` | Sidecar crash handled | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-012` | Sidecar hang handled | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-013` | Timeout/cancellation tested | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-014` | Tool allow-list audited | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-015` | Destructive operations classified | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-016` | Human Change Set boundary tested | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-017` | Nested AI mutation rejected | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-018` | Prompt injection corpus exists | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-019` | Retrieval poisoning corpus exists | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-020` | Recall@K measured | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-021` | MRR measured | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-022` | Offline/no-cloud-fallback claim verified | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-023` | AI privacy claim verified | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-11-024` | Model update regression suite exists | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `EXE-12-001` | Data inventory exists | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-002` | Personal-data inventory exists | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-003` | Data-flow map exists | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-004` | Retention is defined | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-005` | Deletion process exists | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-006` | Access/request process exists | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-007` | Correction process exists | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-008` | Portability/export process exists | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-009` | Subprocessor inventory exists | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-010` | Privacy policy matches implementation | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-011` | Terms match implementation | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-012` | Refund policy matches commerce | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-013` | EULA matches licensing | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-014` | Third-party notices are complete | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-015` | Open-source licenses are archived | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-016` | Model licenses are archived | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-017` | Security contact is published | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-12-018` | Qualified counsel review recorded | P0 | Medium | S0 | G07, G15, G16, G19, G20 | ☐ |
| `EXE-13-001` | Crash IDs are generated | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-002` | OS/app/architecture context is recorded | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-003` | Sensitive creative content is excluded by default | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-004` | Support diagnostic bundle is inspectable | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-005` | Secrets are excluded | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-006` | Crash-loop protection exists | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-007` | Support email is company-owned | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-008` | Knowledge base exists | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-009` | Installation guide exists | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-010` | Recovery guide exists | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-011` | Import/export troubleshooting exists | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-012` | AI troubleshooting exists | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-013` | Windows troubleshooting exists | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-014` | macOS troubleshooting exists | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-015` | Escalation levels exist | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-13-016` | Second support operator can run the workflow | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `EXE-14-001` | Domain is company-owned | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-002` | Registrar recovery is documented | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-003` | DNS recovery is documented | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-004` | HTTPS works | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-005` | SPF works | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-006` | DKIM works | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-007` | DMARC policy is deliberate | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-008` | Privacy page works | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-009` | Terms page works | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-010` | Refund page works | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-011` | Support page works | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-012` | Security disclosure works | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-013` | Downloads are current | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-014` | Screenshots match build | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-015` | Marketing claims pass truth review | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-016` | Transactional and marketing email are separated | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-017` | Bounce/complaint handling exists | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-018` | Review request policy is ethical | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-14-019` | Social accounts have MFA and backup admin | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `EXE-15-001` | Local safety backup behavior is documented | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-002` | Manual backup exists | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-003` | Restore is tested | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-004` | Corrupt backup is detected | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-005` | New-machine restore is tested | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-006` | Cross-platform restore is tested | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-007` | RPO is defined where applicable | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-008` | RTO is defined where applicable | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-009` | Company source code is backed up | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-010` | Release artifacts are backed up | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-011` | Legal records are backed up | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-012` | Runbooks are backed up | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-013` | Founder-unavailable procedure exists | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-014` | GitHub-loss procedure exists | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-015` | Store-account-loss procedure exists | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-016` | Apple-account-loss procedure exists | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-017` | Domain-loss procedure exists | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-15-018` | Annual continuity drill is performed | P1 | Medium | S0 | G03, G04, G06, G14, G18, G20 | ☐ |
| `EXE-16-001` | Release candidate is immutable once approved | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-002` | Changelog is generated | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-003` | Release notes are generated | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-004` | Known issues are documented | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-005` | Checksums are generated | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-006` | SBOM is generated | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-007` | Artifacts are signed | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-008` | Signatures are independently verified | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-009` | Windows artifact is tested | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-010` | macOS artifact is tested | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-011` | Store artifact is tested | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-012` | Upgrade is tested | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-013` | Rollback artifact exists | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-014` | Launch war room exists | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-015` | Post-launch health review exists | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `EXE-16-016` | Recurring maintenance calendar exists | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `BUS-001` | Umbrella-company ownership, product/brand ownership and publisher identity are documented. | P1 | Medium | S0 | G18, G20 | ☐ |
| `BUS-002` | Legal entity, trading name, product name, billing identity and support identity are reconciled. | P1 | Medium | S0 | G18, G20 | ☐ |
| `BUS-003` | Ownership of source code, trademarks, domains, artwork and documentation is documented. | P1 | Medium | S0 | G18, G20 | ☐ |
| `BUS-004` | Founder-only dependencies have a named backup or recovery path. | P1 | Medium | S0 | G18, G20 | ☐ |
| `BUS-005` | Revenue, refunds, fees, taxes and accounting workflow have an owner and evidence trail. | P1 | Medium | S0 | G18, G20 | ☐ |
| `REQ-001` | Every promised capability has one canonical requirement ID in a feature ledger. | P1 | Medium | S0 | G01, G15, G20 | ☐ |
| `REQ-002` | Each requirement records source, priority, owner, state, evidence and applicable gates. | P1 | Medium | S0 | G01, G15, G20 | ☐ |
| `REQ-003` | Requirements support Required, Applicable, N/A, Blocked, Evidence Pending, Pass and Risk Accepted states. | P1 | Medium | S0 | G01, G15, G20 | ☐ |
| `REQ-004` | N/A decisions require a reason; critical N/A decisions require explicit approval. | P1 | Medium | S0 | G01, G15, G20 | ☐ |
| `REQ-005` | Public product claims are traceable to shipped behavior and evidence. | P1 | Medium | S0 | G01, G15, G20 | ☐ |
| `REQ-006` | Launch scope and post-launch backlog are explicitly separated. | P1 | Medium | S0 | G01, G15, G20 | ☐ |
| `ARCH-001` | React remains presentation/client code and never becomes canonical project storage. | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `ARCH-002` | Canonical mutations remain behind the approved Rust application/store/registry boundary. | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `ARCH-003` | Search and AI indexes are derived and rebuildable. | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `ARCH-004` | Platform-specific behavior is isolated behind adapters/services. | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `ARCH-005` | Forbidden dependency directions are mechanically checked. | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `ARCH-006` | Generated bindings/contracts are reproducible and drift is a CI failure. | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `ARCH-007` | Retired LAN collaboration is not reintroduced without a new approved architecture decision. | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `ARCH-008` | Trust boundaries and data flows are versioned architecture artifacts. | P0 | High | S0 | G01, G02, G04, G06, G08, G20 | ☐ |
| `DATA-001` | Every schema change has a forward migration and representative upgrade fixtures. | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `DATA-002` | Migration tests cover fresh, legacy, representative, large and interrupted projects. | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `DATA-003` | Migration failure cannot silently discard canonical data. | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `DATA-004` | Undo/redo semantics survive supported mutations and migrations. | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `DATA-005` | Autosave/checkpoint behavior is safe under process termination. | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `DATA-006` | Restart/reopen proves persisted state before a feature is considered complete. | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `DATA-007` | SQLite integrity and application invariants are checked at recovery boundaries. | P0 | Critical | S0 | G02, G03, G04, G20 | ☐ |
| `PORT-001` | Project format versioning and compatibility policy are explicit. | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `PORT-002` | Project manifest and validation sequence are explicit. | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `PORT-003` | Cross-platform portability is a semantic round trip, not merely a successful open. | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `PORT-004` | Unknown/unsupported constructs are preserved, mapped or warned about explicitly. | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `PORT-005` | Project imports never execute project content as code. | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `PORT-006` | Replacement/import-over-existing behavior is explicit. | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `PORT-007` | Customer recovery on a replacement computer is documented and tested. | P0 | High | S0 | G02, G03, G04, G05, G06, G15, G20 | ☐ |
| `SCR-001` | Screenplay elements have a stable canonical model independent of rendering. | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `SCR-002` | Pagination is deterministic for a pinned renderer/version and covered by fixtures. | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `SCR-003` | Scene numbering, revisions, production locking and page metadata have explicit invariants. | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `SCR-004` | FDX import is previewable and warns about unknown/lossy constructs. | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `SCR-005` | FDX export is round-trip tested. | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `SCR-006` | Fountain, DOCX and PDF capabilities have explicit fidelity expectations. | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `SCR-007` | Unicode, mixed-language text and IME input are tested. | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `SCR-008` | Screenplay PDF output has visual regression coverage. | P1 | Medium | S0 | G02, G04, G05, G12, G13, G15, G19, G20 | ☐ |
| `PROD-001` | Story, scene, screenplay, breakdown, schedule and call-sheet data maintain explicit lineage. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `PROD-002` | Story Board customization and long-form content have defined persistence/export behavior. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `PROD-003` | Breakdown bulk/quick operations preserve canonical data and review state. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `PROD-004` | Catalog, Locations, Cast and Crew references have stable identity/reuse behavior. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `PROD-005` | Schedule covers time, duration, breaks, travel/company moves, wrap and conflicts. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `PROD-006` | DOOD is a deterministic derived view/report. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `PROD-007` | Call Sheets derive from approved production data with explicit manual overrides. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `PROD-008` | Budget scope is explicitly lightweight and not full studio accounting. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `PROD-009` | Production reports are reproducible from canonical state. | P1 | Medium | S0 | G02, G03, G04, G05, G08, G12, G13, G15, G19, G20 | ☐ |
| `AI-001` | Every AI-exposed operation has metadata for capability, mutation class, confirmation and side effects. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-002` | AI cannot bypass canonical mutation APIs or human Change Set approval. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-003` | AI-visible data respects permissions and privacy classification. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-004` | AI retrieval quality has a fixed evaluation corpus and thresholds. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-005` | Model/runtime updates have regression evaluation. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-006` | Prompt injection, retrieval poisoning, tool misuse and privilege escalation are tested. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-007` | Model downloads are verified by trusted metadata/hash/signature before use. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-008` | AI sidecar startup/auth/timeout/cancel/crash/hang/shutdown/orphan cleanup are tested. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-009` | Minimum hardware and performance are measured on supported targets. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `AI-010` | Offline/local-AI claims are proven against actual network/process behavior. | P0 | High | S0 | G04, G06, G07, G08, G09, G13, G14, G15, G20 | ☐ |
| `SEC-001` | Trust boundaries cover WebView, IPC, Rust, filesystem, SQLite, importers, AI, updater and services. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-002` | IPC is allow-listed and capability checked. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-003` | Path handling rejects traversal, symlink escape and platform-dangerous path forms. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-004` | Parsers are bounded for size, depth, decompression and resource use. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-005` | Secrets and credentials are absent from source, artifacts and logs. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-006` | Dependencies have vulnerability and license review with documented exceptions. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-007` | SBOM and release provenance are generated. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-008` | Logs exclude screenplay text, private notes, project contents and credentials by default. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-009` | Security disclosure and severity/triage workflow exist. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `SEC-010` | Critical unresolved security findings block release. | P0 | Critical | S0 | G06, G07, G10, G11, G14, G18, G20 | ☐ |
| `PRIV-001` | A data inventory covers collected, stored, transmitted and derived data. | P0 | Critical | S0 | G20 | ☐ |
| `PRIV-002` | External services have documented purpose, data flow, retention and contractual review. | P0 | Critical | S0 | G20 | ☐ |
| `PRIV-003` | Privacy policy matches actual implementation. | P0 | Critical | S0 | G20 | ☐ |
| `PRIV-004` | Creative content is not sent externally unless explicitly required and disclosed. | P0 | Critical | S0 | G20 | ☐ |
| `PRIV-005` | Applicable privacy requests have an operational owner and workflow. | P0 | Critical | S0 | G20 | ☐ |
| `PRIV-006` | Crash/telemetry data is minimized and redacted. | P0 | Critical | S0 | G20 | ☐ |
| `LEGAL-001` | Terms/EULA, privacy, refund/cancellation and licensing language are consistent with implementation. | P0 | High | S0 | G07, G10, G15, G16, G19, G20 | ☐ |
| `LEGAL-002` | Open-source and third-party notices are complete. | P0 | High | S0 | G07, G10, G15, G16, G19, G20 | ☐ |
| `LEGAL-003` | AI/model/embedding licenses are recorded. | P0 | High | S0 | G07, G10, G15, G16, G19, G20 | ☐ |
| `LEGAL-004` | Marketing claims receive product/legal truth review. | P0 | High | S0 | G07, G10, G15, G16, G19, G20 | ☐ |
| `LEGAL-005` | Trademark/domain/product-name rights are reviewed. | P0 | High | S0 | G07, G10, G15, G16, G19, G20 | ☐ |
| `WIN-001` | Windows clean install, upgrade, uninstall and reinstall are tested on clean machines. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `WIN-002` | Windows signatures and timestamps are independently verified. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `WIN-003` | WebView2 behavior and prerequisites are documented. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `WIN-004` | SmartScreen/Defender behavior is tested from actual release artifacts. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `WIN-005` | High DPI, multi-monitor, IME, Unicode, long paths, external drives and sleep/wake are tested. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `WIN-006` | Store and direct-distribution channels have explicit distinct update/entitlement behavior. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `MAC-001` | macOS clean install, upgrade, uninstall/reinstall and Gatekeeper/quarantine behavior are tested. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `MAC-002` | Supported architectures are explicit and enforced. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `MAC-003` | Developer ID, Hardened Runtime, entitlements, notarization and stapling are verified. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `MAC-004` | Filesystem, case sensitivity, Unicode, permissions, volumes and sleep/wake are tested. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `MAC-005` | Apple account/certificate/notarization recovery has a backup operator. | P0 | High | S0 | G04, G06, G10, G11, G12, G13, G14, G15, G20 | ☐ |
| `STORE-001` | Partner Center/company ownership has at least two authorized administrators. | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `STORE-002` | Publisher/package identity matches the legal company and release pipeline. | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `STORE-003` | Store listing, screenshots, rating, category, regions, pricing, trial and URLs match the release. | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `STORE-004` | Certification testing is repeatable and evidence is archived. | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `STORE-005` | Store install, update, uninstall/reinstall and entitlement restoration are tested. | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `STORE-006` | Store commerce limitations are distinguished from app UX claims. | P0 | High | S0 | G10, G11, G15, G16, G19, G20 | ☐ |
| `COMM-001` | The advertised 7-day trial requires no credit card. | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `COMM-002` | Trial expiry, reinstall, account/device changes, clock tampering and offline behavior are defined. | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `COMM-003` | Purchase, entitlement refresh, restore, cancellation and refund paths are tested. | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `COMM-004` | Discounts/dynamic pricing have one authoritative owner per channel. | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `COMM-005` | Store, website and app pricing/trial/region/currency claims agree. | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `COMM-006` | Commerce outages cannot corrupt project data. | P0 | High | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `UPD-001` | Every release has immutable version, channel, artifact hash, signature/provenance and notes. | P0 | Critical | S0 | G04, G06, G10, G11, G14, G15, G20 | ☐ |
| `UPD-002` | Direct updates require approval and authenticity verification. | P0 | Critical | S0 | G04, G06, G10, G11, G14, G15, G20 | ☐ |
| `UPD-003` | Store/App Store updates use platform-native mechanisms. | P0 | Critical | S0 | G04, G06, G10, G11, G14, G15, G20 | ☐ |
| `UPD-004` | Update failure preserves the prior usable application where technically supported. | P0 | Critical | S0 | G04, G06, G10, G11, G14, G15, G20 | ☐ |
| `UPD-005` | Projects are checkpointed before update and active mutations are drained. | P0 | Critical | S0 | G04, G06, G10, G11, G14, G15, G20 | ☐ |
| `SUP-001` | Customers can obtain support without founder-only intervention. | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `SUP-002` | Support severity, response targets and escalation are defined. | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `SUP-003` | Safe diagnostic bundles can be generated without creative content/secrets. | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `SUP-004` | KB covers installation, update, recovery, import/export, AI, Windows and macOS. | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `SUP-005` | Support can reproduce issues from safe version/build/platform/project-fixture data. | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `SUP-006` | Errors provide actionable recovery guidance. | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `SUP-007` | Crash loops have a safe recovery path. | P1 | Medium | S0 | G07, G14, G15, G17, G20 | ☐ |
| `GTM-001` | Website/domain/DNS ownership and recovery are company-controlled. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-002` | Website download links always target current verified releases. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-003` | Website, Store, app and policies make consistent claims. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-004` | Transactional and marketing email are appropriately separated. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-005` | SPF, DKIM and DMARC are configured and monitored. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-006` | Social accounts have MFA, recovery codes and backup administration. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-007` | Review requests are honest and non-deceptive. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-008` | Content calendar covers launch, education, updates and support education. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-009` | Marketing assets have an owner and source of truth. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `GTM-010` | Analytics/attribution is privacy-reviewed and non-essential collection can be disabled if applicable. | P1 | Medium | S0 | G07, G15, G16, G17, G19, G20 | ☐ |
| `BCP-001` | Critical company assets have independent backups and tested restores. | P1 | Medium | S0 | G20 | ☐ |
| `BCP-002` | Recovery objectives exist for source, domains, store accounts, signing, website, support and release artifacts. | P1 | Medium | S0 | G20 | ☐ |
| `BCP-003` | Loss of GitHub, Microsoft, Apple, domain or email access has a recovery plan. | P1 | Medium | S0 | G20 | ☐ |
| `BCP-004` | A second operator can perform critical release/support/recovery workflows. | P1 | Medium | S0 | G20 | ☐ |
| `BCP-005` | A disaster-recovery drill is completed before launch and recurring thereafter. | P1 | Medium | S0 | G20 | ☐ |
| `OPS-001` | Launch-day war room has primary/backup owners, communication channel, decision authority and stop-ship triggers. | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `OPS-002` | Release evidence is durably stored by immutable release identifier. | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `OPS-003` | Post-launch health review covers crashes, activation, trial conversion, support, refunds, reviews, performance and security. | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `OPS-004` | Recurring maintenance calendar covers dependencies, certificates, domains, policies, backups and vendors. | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |
| `OPS-005` | End-of-life/product-continuity policy protects customer project access and support expectations. | P0 | Medium | S0 | G04, G06, G10, G11, G14, G15, G16, G17, G18, G19, G20 | ☐ |

---

# 5. Domain operating notes

## Architecture & governance
Rules that prevent semantic drift and preserve the durable core.

## Database & migrations
Schema evolution, fixtures, transactions and recovery.

## Project format & portability
Portable project packages, compatibility and semantic round trips.

## Screenplay & writing
Structured screenplay, pagination, revisions and migration.

## Import/export security & reliability
Safe parsers, bounded inputs, transactional import/export.

## Windows
Compatibility, signing, installers, Store/direct channels and OS behavior.

## macOS
Compatibility, signing, notarization, packaging and OS behavior.

## Microsoft Store
Publisher ownership, listing, certification, package and Store lifecycle.

## Trial & commerce
7-day no-card trial, pricing, discounts, entitlement and refunds.

## Security
Threat model, parser safety, secrets, dependencies, signing and disclosure.

## AI
Local model runtime, retrieval, bounded tools, safety, privacy and evaluation.

## Privacy & legal
Data flows, policy truthfulness, licensing and customer rights workflows.

## Crash, diagnostics & support
Customer diagnostics, support operations and escalation.

## Website, email & marketing
Acquisition, content, email, social, reviews and marketing truth.

## Backup & continuity
Customer project recovery and company operational continuity.

## Release & operations
CI/CD, release evidence, launch, monitoring and maintenance.

## Business foundation
Ownership, authority, continuity, accounting and commercial prerequisites.

## Requirements & traceability
Scope, traceability, evidence and change governance.

## Data integrity & persistence
Canonical domain requirements.

## Production workflows
Story, board, breakdown, catalog, schedule, DOOD, call sheets and budget.

## Privacy
Data flows, telemetry and customer-data handling.

## Legal & licensing
Contracts, notices, licenses, trademarks and claim accuracy.

## Commerce & trial
7-day no-card trial, pricing, discounts, entitlement and refunds.

## Updates & installers
Installer trust, updates, rollback and channel behavior.

## Support & customer care
Customer self-service, diagnostics and escalation.

## Backup & business continuity
Canonical domain requirements.

## Release operations
CI/CD, release evidence, launch, monitoring and maintenance.

# 6. Feature-contract template

Create one feature-contract record in the feature ledger for every customer-visible feature and every high-risk internal capability. Do not paste this template repeatedly into the master checklist.

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
| **Tests/evidence** | What proves S4/S5? |

---

# 6A. Detailed product acceptance — screenplay / Final Draft migration

This section is deliberately domain-specific. It does not repeat the generic lifecycle gate; it states what “deep enough” means for the screenplay product.

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

# 6B. Detailed product acceptance — Story Board / Breakdown / Production

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

# 6C. Detailed product acceptance — Schedule / DOOD / Call Sheets / Budget

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

# 6D. Detailed product acceptance — Appearance Theme System (47A)

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

# 7. AI-assisted development control

- [ ] Before coding: list affected modules, invariants, files, operations, migrations, tests and platform impact.
- [ ] Implement the smallest vertical slice.
- [ ] Run format/lint/typecheck/tests/generated-binding checks.
- [ ] Inspect semantic diff for Store bypass, direct DB writes, path changes, permission changes, migration changes, AI exposure and export changes.
- [ ] Run lifecycle tests for create/save/restart/undo/redo/delete/restore/export/import where applicable.
- [ ] Run targeted security tests for parser/path/package/AI/process/external-link/update changes.
- [ ] V4–V8 changes never merge solely because the feature works.
- [ ] Record how AI sees and does not see the new operation.

The supplied vibe-coding document explicitly recommends this workflow and PR gates. L1-L3

# 8. End-to-end and failure testing

## Mandatory customer journey

`launch → create project → story → scene → screenplay → breakdown → schedule → call sheet → save → close → relaunch → reopen → edit → undo → redo → export → re-import → continue`

Run it on every supported OS target. This journey is explicitly required by the OS/security plan. L1-L1

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

The OS/security plan explicitly requires storage, process, network and malformed-file failure injection. L1-L1

# 9. Windows + macOS release matrix

| Capability | Windows x64 | macOS arm64 | macOS x64 if supported | Evidence |
|---|---:|---:|---:|---|
| Clean install | ☐ | ☐ | ☐ | |
| Upgrade | ☐ | ☐ | ☐ | |
| Uninstall/reinstall | ☐ | ☐ | ☐ | |
| Project create/open/close | ☐ | ☐ | ☐ | |
| Migration | ☐ | ☐ | ☐ | |
| Autosave/recovery | ☐ | ☐ | ☐ | |
| Undo/redo | ☐ | ☐ | ☐ | |
| Trash/restore | ☐ | ☐ | ☐ | |
| Files | ☐ | ☐ | ☐ | |
| Search | ☐ | ☐ | ☐ | |
| AI runtime | ☐ | ☐ | ☐ | |
| Import/export | ☐ | ☐ | ☐ | |
| High DPI/display scaling | ☐ | ☐ | ☐ | |
| IME/Unicode | ☐ | ☐ | ☐ | |
| External drives | ☐ | ☐ | ☐ | |
| Permissions | ☐ | ☐ | ☐ | |
| Sleep/wake | ☐ | ☐ | ☐ | |
| Crash recovery | ☐ | ☐ | ☐ | |
| Offline behavior | ☐ | ☐ | ☐ | |
| Update | ☐ | ☐ | ☐ | |
| Artifact verification | ☐ | ☐ | ☐ | |

The OS/security plan requires Windows x64, macOS arm64 and macOS x64 where supported, with equivalent lifecycle tests. L1-L1

# 10. Microsoft Store launch runbook

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

# 11. Commerce and 7-day no-card trial

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

# 12. Website + customer email + social

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

# 13. Support and diagnostics

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

# 14. Privacy, legal, security and supply chain

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

The OS/security plan requires signed model/update handling, bounded import security, protected secrets, privacy-safe logging and cross-platform security invariants. L2-L6

# 15. Backup, disaster recovery and founder-independence

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

# 16. Release engineering and CI/CD

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

The supplied vibe-coding plan defines compile → behavior → architecture → data → security → platform → release gates. L1-L3

# 17. Launch timeline

| **T-180→T-121** | Ownership, accounts, scope, architecture freeze, feature ledger, evidence system. |
| **T-120→T-91** | Core feature integration, migration fixtures, import/export corpus, OS CI, security model, support foundation. |
| **T-90→T-61** | Screenplay/production hardening, AI evaluation, themes/visual regression, installers, website, policies. |
| **T-60→T-31** | Release candidates, clean-machine tests, Store preparation, support/restore drills, marketing assets. |
| **T-30→T-15** | Certification/submission, commercial validation, email deliverability, final security and docs. |
| **T-14→T-7** | Full E2E, failure injection, platform parity, public-surface consistency audit, war-room rehearsal. |
| **T-6→T-1** | Final evidence, signatures/notarization/Store package, backups, access verification, GO/NO-GO. |
| **T0** | Publish, smoke-test install/trial/purchase/update, monitor incidents and support. |
| **T+1→T+7** | Daily health review; urgent reliability/security fixes only under release controls. |
| **T+8→T+30** | First product health review, top bugs, support/knowledge improvements, security review. |
| **T+31→T+90** | Quarterly operating cadence, continuity drill, roadmap reprioritization. |

# 18. Launch-day war room and stop-ship

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

# 19. Post-launch operations

| Cadence | Required review |
|---|---|
| Daily first week | Crashes, activation/trial, Store health, support, incidents, reviews. |
| Weekly | Top bugs, customer friction, support backlog, conversion, refunds, release pipeline. |
| Monthly | Dependencies, security advisories, certificates, domains, vendors, costs, email health. |
| Quarterly | Backups/restore, account access, continuity, privacy/legal consistency, roadmap. |
| Per release | Full release gates and evidence archive. |
| Annually | Security/legal review, DR drill, vendor review, product continuity review. |

# 20. Automation register

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

# 21. Evidence index template

| Evidence ID | Requirement IDs | Artifact/test | Version/commit | Date | Owner | Result | Location |
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

# 22. Risk register template

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

# 23. Vendor/dependency register

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

# 24. RACI / continuity matrix

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

# 25. Product health scorecard

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
| Quality | P0/P1 defects | | | | |
| Quality | Regression escape | | | | |
| Quality | Rollback count | | | | |
| Security | Open critical findings | | | | |
| Security | Dependency exposure | | | | |
| Security | Security reports | | | | |
| Operations | Backup verification | | | | |
| Operations | Expiry risk | | | | |
| Operations | Single-person dependencies | | | | |

# 26. Final adversarial audit

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
- [ ] Are all P0 requirements PASS or explicitly blocked from release?
- [ ] Does every PASS have evidence?
- [ ] Would operations survive six months without the original developer?

# 27. Master release gate

- [ ] All applicable P0 requirements are S5+.
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

# 28. Maintenance calendar

| Frequency | Maintenance |
|---|---|
| Per PR | Quality, architecture, security and binding gates. |
| Per release | Full release evidence, installers, Store/site consistency. |
| Weekly | Support, crash, review and incident review. |
| Monthly | Dependencies, certificates, domains, vendors, costs and email health. |
| Quarterly | Backup restore, account access, continuity and policy consistency. |
| Annually | Security, legal, DR, vendor and product-continuity review. |

# 29. Source-of-truth hierarchy

1. **Protected architecture/security invariants**
2. **Current approved product specification and decisions**
3. **Canonical feature ledger/requirement register**
4. **Current implementation evidence/tests**
5. **Current customer-facing release declaration**
6. **Supporting manuals/historical plans**
7. **Ideas/prototypes/obsolete documents**

The completion plan explicitly rejects “finish all old items, then redesign” and recommends dependency-aware reconciliation by module. L1-L2

# 30. Anti-overengineering rule

- [ ] Prefer automation over recurring manual memory.
- [ ] Prefer evidence over confidence.
- [ ] Prefer one canonical requirement over multiple copies.
- [ ] Prefer safe failure over silent partial success.
- [ ] Prefer reversible operations over destructive shortcuts.
- [ ] Prefer platform adapters over duplicated business logic.
- [ ] Prefer explicit launch scope over feature creep.
- [ ] Do not add enterprise-scale infrastructure unless a real product requirement justifies it.

# 31. Final operating philosophy

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

# Appendix A — Preserved EXE traceability

The prior master contained **342 checkbox lines**, of which **285 were `EXE-*` execution requirements**; the remaining checklist lines were shared operational templates/sign-off material. All 285 `EXE-*` requirements are preserved as canonical rows below, while the shared boilerplate is represented once in the gates/templates.

| EXE ID | Canonical requirement | Domain |
|---|---|---|
| `EXE-01-001` | React never owns canonical project truth | Architecture & governance |
| `EXE-01-002` | React never opens SQLite directly | Architecture & governance |
| `EXE-01-003` | React never writes arbitrary project files | Architecture & governance |
| `EXE-01-004` | All canonical mutations pass through the approved Rust mutation pipeline | Architecture & governance |
| `EXE-01-005` | Store::mutate or approved equivalent remains authoritative | Architecture & governance |
| `EXE-01-006` | Registry is a strict allow-list | Architecture & governance |
| `EXE-01-007` | Search is derived/rebuildable | Architecture & governance |
| `EXE-01-008` | AI indexes are derived/rebuildable | Architecture & governance |
| `EXE-01-009` | AI cannot directly mutate canonical state | Architecture & governance |
| `EXE-01-010` | AI proposals require human Change Set approval | Architecture & governance |
| `EXE-01-011` | AI cannot recursively/nestedly grant itself mutation authority | Architecture & governance |
| `EXE-01-012` | Platform differences are hidden behind adapters | Architecture & governance |
| `EXE-01-013` | No ADR-0007 LAN collaboration reintroduction | Architecture & governance |
| `EXE-01-014` | Generated bindings are reproducible | Architecture & governance |
| `EXE-01-015` | Forbidden dependency directions are mechanically checked | Architecture & governance |
| `EXE-02-001` | Every schema change has a numbered migration | Database & migrations |
| `EXE-02-002` | Migration is transactional | Database & migrations |
| `EXE-02-003` | Migration has fresh-database fixture | Database & migrations |
| `EXE-02-004` | Migration has representative legacy fixture | Database & migrations |
| `EXE-02-005` | Migration has largest realistic fixture | Database & migrations |
| `EXE-02-006` | Migration has interrupted-process fixture | Database & migrations |
| `EXE-02-007` | Migration has disk-full fixture | Database & migrations |
| `EXE-02-008` | Migration has permission-denied fixture | Database & migrations |
| `EXE-02-009` | Migration has post-migration integrity_check | Database & migrations |
| `EXE-02-010` | Migration has search-index rebuild test | Database & migrations |
| `EXE-02-011` | Migration has AI-index rebuild test | Database & migrations |
| `EXE-02-012` | Migration preserves undo/redo semantics | Database & migrations |
| `EXE-02-013` | Migration preserves exportability | Database & migrations |
| `EXE-02-014` | Migration has documented downgrade policy | Database & migrations |
| `EXE-03-001` | Project format has explicit version | Project format & portability |
| `EXE-03-002` | Project has manifest | Project format & portability |
| `EXE-03-003` | Project records schema version | Project format & portability |
| `EXE-03-004` | Project records application compatibility | Project format & portability |
| `EXE-03-005` | Project package validates before import | Project format & portability |
| `EXE-03-006` | ZIP extraction rejects traversal | Project format & portability |
| `EXE-03-007` | Symlink escape is rejected | Project format & portability |
| `EXE-03-008` | Unicode paths are supported | Project format & portability |
| `EXE-03-009` | Long paths are tested | Project format & portability |
| `EXE-03-010` | Missing assets have deterministic behavior | Project format & portability |
| `EXE-03-011` | Windows-to-macOS round trip passes | Project format & portability |
| `EXE-03-012` | macOS-to-Windows round trip passes | Project format & portability |
| `EXE-03-013` | Customer can export important creative work | Project format & portability |
| `EXE-03-014` | Customer can recover after reinstall | Project format & portability |
| `EXE-03-015` | Customer can recover after machine replacement | Project format & portability |
| `EXE-04-001` | Element model is structured | Screenplay & writing |
| `EXE-04-002` | Element progression is deterministic | Screenplay & writing |
| `EXE-04-003` | Pagination is deterministic | Screenplay & writing |
| `EXE-04-004` | Dialogue continuation is tested | Screenplay & writing |
| `EXE-04-005` | Parenthetical behavior is tested | Screenplay & writing |
| `EXE-04-006` | Scene numbering is tested | Screenplay & writing |
| `EXE-04-007` | Revision colors are tested | Screenplay & writing |
| `EXE-04-008` | Locked production draft is tested | Screenplay & writing |
| `EXE-04-009` | Title page is tested | Screenplay & writing |
| `EXE-04-010` | Find/replace is tested | Screenplay & writing |
| `EXE-04-011` | Unicode is tested | Screenplay & writing |
| `EXE-04-012` | Multilingual content is tested | Screenplay & writing |
| `EXE-04-013` | FDX import preview is tested | Screenplay & writing |
| `EXE-04-014` | FDX export is re-importable | Screenplay & writing |
| `EXE-04-015` | Unknown constructs are preserved/mapped/warned | Screenplay & writing |
| `EXE-04-016` | Fountain round trip is tested | Screenplay & writing |
| `EXE-04-017` | DOCX compatibility is documented | Screenplay & writing |
| `EXE-04-018` | PDF import is explicitly best-effort | Screenplay & writing |
| `EXE-04-019` | PDF export visual regression is automated | Screenplay & writing |
| `EXE-04-020` | Screenplay exports remain unbranded | Screenplay & writing |
| `EXE-05-001` | Malformed ZIP cannot escape destination | Import/export security & reliability |
| `EXE-05-002` | Malformed XML cannot crash process unsafely | Import/export security & reliability |
| `EXE-05-003` | Malformed PDF is safely rejected or degraded | Import/export security & reliability |
| `EXE-05-004` | Malformed DOCX is safely rejected or degraded | Import/export security & reliability |
| `EXE-05-005` | Oversized files are bounded | Import/export security & reliability |
| `EXE-05-006` | Decompression bombs are bounded | Import/export security & reliability |
| `EXE-05-007` | Import is transactional | Import/export security & reliability |
| `EXE-05-008` | Import preview exposes warnings | Import/export security & reliability |
| `EXE-05-009` | Import preserves original source | Import/export security & reliability |
| `EXE-05-010` | Export uses canonical state | Import/export security & reliability |
| `EXE-05-011` | Export failure cannot mutate canonical state | Import/export security & reliability |
| `EXE-05-012` | Partial output is cleaned up | Import/export security & reliability |
| `EXE-05-013` | Existing destination behavior is explicit | Import/export security & reliability |
| `EXE-05-014` | Unicode destination paths work | Import/export security & reliability |
| `EXE-05-015` | Permission failure is recoverable | Import/export security & reliability |
| `EXE-06-001` | Supported Windows versions are documented | Windows |
| `EXE-06-002` | Clean install tested | Windows |
| `EXE-06-003` | Upgrade tested | Windows |
| `EXE-06-004` | Uninstall tested | Windows |
| `EXE-06-005` | Reinstall tested | Windows |
| `EXE-06-006` | WebView2 behavior verified | Windows |
| `EXE-06-007` | AuthentiCode signature verified | Windows |
| `EXE-06-008` | Timestamp verified | Windows |
| `EXE-06-009` | SmartScreen behavior tested | Windows |
| `EXE-06-010` | Windows Defender interaction tested | Windows |
| `EXE-06-011` | High DPI tested | Windows |
| `EXE-06-012` | Multiple monitors tested | Windows |
| `EXE-06-013` | Unicode/IME tested | Windows |
| `EXE-06-014` | Long paths tested | Windows |
| `EXE-06-015` | External drive removal tested | Windows |
| `EXE-06-016` | Read-only folder tested | Windows |
| `EXE-06-017` | Network-share behavior explicitly defined | Windows |
| `EXE-06-018` | Sleep/wake tested | Windows |
| `EXE-06-019` | AI sidecar process cleanup tested | Windows |
| `EXE-06-020` | Offline mode tested | Windows |
| `EXE-07-001` | Supported macOS versions are documented | macOS |
| `EXE-07-002` | Apple Silicon tested | macOS |
| `EXE-07-003` | Intel tested if supported | macOS |
| `EXE-07-004` | Developer ID signing verified | macOS |
| `EXE-07-005` | Hardened Runtime enabled | macOS |
| `EXE-07-006` | Entitlements minimized | macOS |
| `EXE-07-007` | Secure timestamp verified | macOS |
| `EXE-07-008` | Notarization verified | macOS |
| `EXE-07-009` | Ticket stapled where applicable | macOS |
| `EXE-07-010` | Gatekeeper tested | macOS |
| `EXE-07-011` | Quarantine test performed | macOS |
| `EXE-07-012` | DMG/PKG tested as applicable | macOS |
| `EXE-07-013` | External drives tested | macOS |
| `EXE-07-014` | File permissions tested | macOS |
| `EXE-07-015` | Sleep/wake tested | macOS |
| `EXE-07-016` | Unicode/case-sensitivity assumptions tested | macOS |
| `EXE-07-017` | AI sidecar lifecycle tested | macOS |
| `EXE-07-018` | Updater signing tested | macOS |
| `EXE-07-019` | Clean-machine release test completed | macOS |
| `EXE-07-020` | Developer account recovery documented | macOS |
| `EXE-08-001` | Partner Center company ownership verified | Microsoft Store |
| `EXE-08-002` | Backup administrator exists | Microsoft Store |
| `EXE-08-003` | Publisher identity verified | Microsoft Store |
| `EXE-08-004` | Package identity verified | Microsoft Store |
| `EXE-08-005` | Store listing is complete | Microsoft Store |
| `EXE-08-006` | Screenshots match build | Microsoft Store |
| `EXE-08-007` | Description matches build | Microsoft Store |
| `EXE-08-008` | Privacy URL works | Microsoft Store |
| `EXE-08-009` | Support URL works | Microsoft Store |
| `EXE-08-010` | Age rating completed | Microsoft Store |
| `EXE-08-011` | Category completed | Microsoft Store |
| `EXE-08-012` | Pricing verified | Microsoft Store |
| `EXE-08-013` | Regional availability verified | Microsoft Store |
| `EXE-08-014` | Trial configuration verified | Microsoft Store |
| `EXE-08-015` | Certification notes prepared | Microsoft Store |
| `EXE-08-016` | Windows App Certification Kit run where applicable | Microsoft Store |
| `EXE-08-017` | Clean Store install tested | Microsoft Store |
| `EXE-08-018` | Store update tested | Microsoft Store |
| `EXE-08-019` | Store uninstall/reinstall tested | Microsoft Store |
| `EXE-08-020` | Certification evidence archived | Microsoft Store |
| `EXE-09-001` | 7-day trial starts correctly | Trial & commerce |
| `EXE-09-002` | No credit card is required | Trial & commerce |
| `EXE-09-003` | Trial expiry is deterministic | Trial & commerce |
| `EXE-09-004` | Offline trial behavior is defined | Trial & commerce |
| `EXE-09-005` | Reinstall behavior is defined | Trial & commerce |
| `EXE-09-006` | Clock tampering behavior is defined | Trial & commerce |
| `EXE-09-007` | Purchase during trial works | Trial & commerce |
| `EXE-09-008` | Entitlement refresh works | Trial & commerce |
| `EXE-09-009` | Restore purchase works | Trial & commerce |
| `EXE-09-010` | Refund path is documented | Trial & commerce |
| `EXE-09-011` | Cancellation path is documented | Trial & commerce |
| `EXE-09-012` | Store outage behavior is defined | Trial & commerce |
| `EXE-09-013` | Pricing is reconciled with finance | Trial & commerce |
| `EXE-09-014` | Discount expiry has owner | Trial & commerce |
| `EXE-09-015` | Commercial claims match actual flow | Trial & commerce |
| `EXE-10-001` | Threat model exists | Security |
| `EXE-10-002` | IPC allow-list audited | Security |
| `EXE-10-003` | Filesystem scope audited | Security |
| `EXE-10-004` | Path traversal rejected | Security |
| `EXE-10-005` | Symlink escapes rejected | Security |
| `EXE-10-006` | Command injection tested | Security |
| `EXE-10-007` | SQL injection tested | Security |
| `EXE-10-008` | URL handling tested | Security |
| `EXE-10-009` | XML parser tested | Security |
| `EXE-10-010` | ZIP extraction tested | Security |
| `EXE-10-011` | PDF parser tested | Security |
| `EXE-10-012` | DOCX parser tested | Security |
| `EXE-10-013` | Fuzzing corpus exists | Security |
| `EXE-10-014` | Secret scan is clean | Security |
| `EXE-10-015` | Dependency scan reviewed | Security |
| `EXE-10-016` | License scan reviewed | Security |
| `EXE-10-017` | SBOM generated | Security |
| `EXE-10-018` | Signing keys protected | Security |
| `EXE-10-019` | Updater signatures verified | Security |
| `EXE-10-020` | Security disclosure channel tested | Security |
| `EXE-11-001` | Model license recorded | AI |
| `EXE-11-002` | Model checksum recorded | AI |
| `EXE-11-003` | Embedding license recorded | AI |
| `EXE-11-004` | Embedding checksum recorded | AI |
| `EXE-11-005` | Model download integrity verified | AI |
| `EXE-11-006` | Interrupted download tested | AI |
| `EXE-11-007` | Missing model handled | AI |
| `EXE-11-008` | Minimum hardware documented | AI |
| `EXE-11-009` | CPU fallback tested | AI |
| `EXE-11-010` | Vulkan path tested where supported | AI |
| `EXE-11-011` | Sidecar crash handled | AI |
| `EXE-11-012` | Sidecar hang handled | AI |
| `EXE-11-013` | Timeout/cancellation tested | AI |
| `EXE-11-014` | Tool allow-list audited | AI |
| `EXE-11-015` | Destructive operations classified | AI |
| `EXE-11-016` | Human Change Set boundary tested | AI |
| `EXE-11-017` | Nested AI mutation rejected | AI |
| `EXE-11-018` | Prompt injection corpus exists | AI |
| `EXE-11-019` | Retrieval poisoning corpus exists | AI |
| `EXE-11-020` | Recall@K measured | AI |
| `EXE-11-021` | MRR measured | AI |
| `EXE-11-022` | Offline/no-cloud-fallback claim verified | AI |
| `EXE-11-023` | AI privacy claim verified | AI |
| `EXE-11-024` | Model update regression suite exists | AI |
| `EXE-12-001` | Data inventory exists | Privacy & legal |
| `EXE-12-002` | Personal-data inventory exists | Privacy & legal |
| `EXE-12-003` | Data-flow map exists | Privacy & legal |
| `EXE-12-004` | Retention is defined | Privacy & legal |
| `EXE-12-005` | Deletion process exists | Privacy & legal |
| `EXE-12-006` | Access/request process exists | Privacy & legal |
| `EXE-12-007` | Correction process exists | Privacy & legal |
| `EXE-12-008` | Portability/export process exists | Privacy & legal |
| `EXE-12-009` | Subprocessor inventory exists | Privacy & legal |
| `EXE-12-010` | Privacy policy matches implementation | Privacy & legal |
| `EXE-12-011` | Terms match implementation | Privacy & legal |
| `EXE-12-012` | Refund policy matches commerce | Privacy & legal |
| `EXE-12-013` | EULA matches licensing | Privacy & legal |
| `EXE-12-014` | Third-party notices are complete | Privacy & legal |
| `EXE-12-015` | Open-source licenses are archived | Privacy & legal |
| `EXE-12-016` | Model licenses are archived | Privacy & legal |
| `EXE-12-017` | Security contact is published | Privacy & legal |
| `EXE-12-018` | Qualified counsel review recorded | Privacy & legal |
| `EXE-13-001` | Crash IDs are generated | Crash, diagnostics & support |
| `EXE-13-002` | OS/app/architecture context is recorded | Crash, diagnostics & support |
| `EXE-13-003` | Sensitive creative content is excluded by default | Crash, diagnostics & support |
| `EXE-13-004` | Support diagnostic bundle is inspectable | Crash, diagnostics & support |
| `EXE-13-005` | Secrets are excluded | Crash, diagnostics & support |
| `EXE-13-006` | Crash-loop protection exists | Crash, diagnostics & support |
| `EXE-13-007` | Support email is company-owned | Crash, diagnostics & support |
| `EXE-13-008` | Knowledge base exists | Crash, diagnostics & support |
| `EXE-13-009` | Installation guide exists | Crash, diagnostics & support |
| `EXE-13-010` | Recovery guide exists | Crash, diagnostics & support |
| `EXE-13-011` | Import/export troubleshooting exists | Crash, diagnostics & support |
| `EXE-13-012` | AI troubleshooting exists | Crash, diagnostics & support |
| `EXE-13-013` | Windows troubleshooting exists | Crash, diagnostics & support |
| `EXE-13-014` | macOS troubleshooting exists | Crash, diagnostics & support |
| `EXE-13-015` | Escalation levels exist | Crash, diagnostics & support |
| `EXE-13-016` | Second support operator can run the workflow | Crash, diagnostics & support |
| `EXE-14-001` | Domain is company-owned | Website, email & marketing |
| `EXE-14-002` | Registrar recovery is documented | Website, email & marketing |
| `EXE-14-003` | DNS recovery is documented | Website, email & marketing |
| `EXE-14-004` | HTTPS works | Website, email & marketing |
| `EXE-14-005` | SPF works | Website, email & marketing |
| `EXE-14-006` | DKIM works | Website, email & marketing |
| `EXE-14-007` | DMARC policy is deliberate | Website, email & marketing |
| `EXE-14-008` | Privacy page works | Website, email & marketing |
| `EXE-14-009` | Terms page works | Website, email & marketing |
| `EXE-14-010` | Refund page works | Website, email & marketing |
| `EXE-14-011` | Support page works | Website, email & marketing |
| `EXE-14-012` | Security disclosure works | Website, email & marketing |
| `EXE-14-013` | Downloads are current | Website, email & marketing |
| `EXE-14-014` | Screenshots match build | Website, email & marketing |
| `EXE-14-015` | Marketing claims pass truth review | Website, email & marketing |
| `EXE-14-016` | Transactional and marketing email are separated | Website, email & marketing |
| `EXE-14-017` | Bounce/complaint handling exists | Website, email & marketing |
| `EXE-14-018` | Review request policy is ethical | Website, email & marketing |
| `EXE-14-019` | Social accounts have MFA and backup admin | Website, email & marketing |
| `EXE-15-001` | Local safety backup behavior is documented | Backup & continuity |
| `EXE-15-002` | Manual backup exists | Backup & continuity |
| `EXE-15-003` | Restore is tested | Backup & continuity |
| `EXE-15-004` | Corrupt backup is detected | Backup & continuity |
| `EXE-15-005` | New-machine restore is tested | Backup & continuity |
| `EXE-15-006` | Cross-platform restore is tested | Backup & continuity |
| `EXE-15-007` | RPO is defined where applicable | Backup & continuity |
| `EXE-15-008` | RTO is defined where applicable | Backup & continuity |
| `EXE-15-009` | Company source code is backed up | Backup & continuity |
| `EXE-15-010` | Release artifacts are backed up | Backup & continuity |
| `EXE-15-011` | Legal records are backed up | Backup & continuity |
| `EXE-15-012` | Runbooks are backed up | Backup & continuity |
| `EXE-15-013` | Founder-unavailable procedure exists | Backup & continuity |
| `EXE-15-014` | GitHub-loss procedure exists | Backup & continuity |
| `EXE-15-015` | Store-account-loss procedure exists | Backup & continuity |
| `EXE-15-016` | Apple-account-loss procedure exists | Backup & continuity |
| `EXE-15-017` | Domain-loss procedure exists | Backup & continuity |
| `EXE-15-018` | Annual continuity drill is performed | Backup & continuity |
| `EXE-16-001` | Release candidate is immutable once approved | Release & operations |
| `EXE-16-002` | Changelog is generated | Release & operations |
| `EXE-16-003` | Release notes are generated | Release & operations |
| `EXE-16-004` | Known issues are documented | Release & operations |
| `EXE-16-005` | Checksums are generated | Release & operations |
| `EXE-16-006` | SBOM is generated | Release & operations |
| `EXE-16-007` | Artifacts are signed | Release & operations |
| `EXE-16-008` | Signatures are independently verified | Release & operations |
| `EXE-16-009` | Windows artifact is tested | Release & operations |
| `EXE-16-010` | macOS artifact is tested | Release & operations |
| `EXE-16-011` | Store artifact is tested | Release & operations |
| `EXE-16-012` | Upgrade is tested | Release & operations |
| `EXE-16-013` | Rollback artifact exists | Release & operations |
| `EXE-16-014` | Launch war room exists | Release & operations |
| `EXE-16-015` | Post-launch health review exists | Release & operations |
| `EXE-16-016` | Recurring maintenance calendar exists | Release & operations |

# Appendix B — Recommended dependency-aware implementation order

| **Phase 0** | Baseline freeze, feature ledger, architecture lint, fixtures and evidence system. |
| **Phase 1** | Screenplay trust: structured model, pagination, revisions, FDX/Fountain/PDF/DOCX boundaries and round trips. |
| **Phase 2** | Writer comfort, appearance/theme and visual regression. |
| **Phase 3** | Story Board depth and lineage. |
| **Phase 4** | Breakdown/catalog/locations/cast/crew depth. |
| **Phase 5** | Schedule + DOOD + availability/conflicts. |
| **Phase 6** | Call Sheets + production documents + lightweight budget. |
| **Phase 7** | Import/export hardening and package portability. |
| **Phase 8** | AI evaluation, security hardening, minimum-spec performance and tool coverage. |
| **Phase 9** | Windows/macOS production hardening, installers, signing and clean-machine tests. |
| **Phase 10** | Microsoft Store certification/submission and direct distribution readiness. |
| **Phase 11** | Website, support, email, privacy/legal, marketing and social launch system. |
| **Phase 12** | Full release candidate, failure injection, launch war room and final audit. |
| **Phase 13** | Launch and first 90 days of measured operations. |

The supplied execution plan uses this dependency-aware philosophy and explicitly places lifecycle, import/export, AI evaluation, rendering regression, OS CI and release gates before the final feature rush. L1-L2

# Appendix C — Release evidence package

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

# Appendix D — Final sign-off

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

# Appendix E — Maintenance of this document

- [ ] When a requirement changes, update its canonical row first.
- [ ] When a cross-cutting rule changes, update the gate catalog once.
- [ ] Do not duplicate a requirement because multiple teams touch it.
- [ ] Add a new ID for a materially new promise; supersede old IDs with traceable decisions rather than deleting history.
- [ ] At every release, regenerate evidence index and run the adversarial audit.

**End of revised master checklist.**


# Appendix F: Additions from the intake notes (2026-10-05)

Requirements present in the owner's notes (`docs/archive/notes/store-certification-and-launch-gap-notes.md`) or in the Store audit and not in the register above. IDs are new and stable. States start at S0. Priorities: P0 blocks Store submission, P1 blocks launch quality.

| ID | Requirement | Priority | Masterplan |
|---|---|---|---|
| `STORE-007` | In-app way to report inappropriate AI output (Store policy for live generative AI); never sends project content automatically | P0 | M9.13 |
| `STORE-008` | Store listing discloses live generative AI and the optional on-demand download | P0 | M13.04 |
| `STORE-009` | Age rating (IARC) answered accurately for screenplays with mature themes or violence | P0 | M13.03 |
| `STORE-010` | Notes for Certification: no login needed, how to test optional AI, network use, capability justification | P0 | M13.06 |
| `STORE-011` | Clean first launch on a bare machine or Windows Sandbox; no locally installed runtime dependency | P0 | M11.08, M11.09 |
| `STORE-012` | Manifest capabilities limited to those used (runFullTrust, internetClient) with written justification | P0 | M11.02 |
| `STORE-013` | At least one clear, uncropped screenshot of the real UI from the release candidate | P0 | M13.02 |
| `STORE-014` | Windows App Certification Kit report (zero failures) archived for the exact package | P0 | M11.07 |
| `STORE-015` | Supported Windows versions in manifest, listing and docs agree with what was tested | P0 | M0.27 |
| `STORE-016` | Store policy version re-checked on the submission date | P1 | M13.11 |
| `BETA-001` | Real-filmmaker beta on a release candidate, with feedback channel and a closed list of findings | P0 | M10.09 |
| `BETA-002` | Windows hardware matrix (minimum, recommended, discrete GPU, high DPI, multi-monitor) | P0 | M9.06, M10.04 |
| `GTM-011` | Launch trailer and Store media | P1 | M14.06 |
| `GTM-012` | Founder story and public narrative | P1 | M14.06 |
| `GTM-013` | 30-day content bank | P1 | M14.06 |
| `GTM-014` | Creator outreach list; community strategy (Reddit, Discord partnerships) that follows each community rules | P1 | M14.06 |
| `GTM-015` | Website SEO basics and social account reservation | P1 | M12.07, M14.06 |
| `SUP-008` | Customer feedback system (in addition to support) | P1 | M14.07 |
| `SUP-009` | Status and outage communication channel | P1 | M14.07 |
| `OPS-006` | Post-launch incident response runbook | P0 | M14.05 |
| `OPS-007` | Versioning strategy (SemVer app version, four-part MSIX version, schema versions) and changelog | P0 | M11.03, M13.07 |
| `OPS-008` | Localization-ready architecture confirmed (English-only v1) | P1 | M10.10 |
| `OPS-009` | Public documentation and release notes | P1 | M10.13, M13.07 |
| `AI-011` | Store build bundles the runtime; weights downloaded on user click (ADR-0020) | P0 | M9.11 |
| `DATA-008` | Multi-operation AI Change Set apply is crash-safe (ADR-0019) | P0 | M1.01, M1.02 |
| `REL-011` | Desktop E2E is a required release gate | P0 | M1.12 |
| `REL-012` | Supply-chain checks (cargo deny, npm audit, licence inventory) pass without bypass | P0 | M0.21 |
