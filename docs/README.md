# OpenFrame Studio documentation map

> Rewritten 2026-10-15 around the three chapters and the owner's 25 decisions. Start here. It says where every document
> lives, which one wins when two disagree, what is current and what is history, and what must be updated when the code
> changes. The earlier map is in [the archive](archive/2026-10-15-documentation-map-before-restructure.md).

## 0. Start here

| If you want to know | Read |
|---|---|
| What OpenFrame is, who it is for, the chapters | [product/vision-and-scope.md](product/vision-and-scope.md) |
| What to do next | [implementation/handoff.md](implementation/handoff.md), then [roadmap/README.md](roadmap/README.md) |
| The Windows plan, ten phases | [roadmap/chapter-1a-windows-phases.md](roadmap/chapter-1a-windows-phases.md) |
| The Mac plan, ten phases | [roadmap/chapter-1b-mac-phases.md](roadmap/chapter-1b-mac-phases.md) |
| How it is built, in simple words | [architecture/README.md](architecture/README.md), then the [report](reports/2026-10-09-project-retrospection-and-audit.md) (a snapshot) |
| What works and how well it is proved | [implementation/feature-status.md](implementation/feature-status.md) |
| How we test and how the pilot runs | [testing/README.md](testing/README.md), [testing/internal-test-plan.md](testing/internal-test-plan.md) |
| How to keep the machine and the repository tidy | [development-storage-policy.md](development-storage-policy.md) |
| Why something was decided | [adr/README.md](adr/README.md) |
| What only the owner can decide or do | [implementation/owner-decisions.md](implementation/owner-decisions.md) |

## 1. Layout

```text
README.md  CONTRIBUTING.md  DEVELOPMENT.md  CLAUDE.md  LICENSE-PENDING.md  THIRD_PARTY_LICENSES.md   (repository root: only these)
docs/
  README.md                  this map
  product/                   vision and scope (audience, chapters, claims rule)
  roadmap/                   chapter plans: overview, Windows phases, Mac phases, website, stores; area-to-phase map
  architecture/              architecture in one page
  testing/                   strategy, internal pilot plan, bug report template
  reports/                   dated reports (the 2026-10-09 retrospection and audit)
  specs/                     product and engineering specifications (+ README with the status of each)
  adr/                       decisions; README is the decision register
  engineering/               implementation documents 11 to 39, AI tool coverage (generated)
  implementation/            build plan, feature status, owner decisions, handoff, progress and reconciliation logs
  release/                   launch readiness register, evidence folders
  store/                     Microsoft Store listing and privacy policy drafts (Chapter 3)
  website/                   website source notes and supplied prompts (Chapter 2)
  manual/                    user manual (.docx, .pdf), manual testing guide (.docx, .csv), test-data/ sample files
  audits/                    dated evidence snapshots (do not edit)
  archive/                   history; README is the archive index (do not edit the files)
  (spec-digest/ moved to archive/2026-09-spec-digest/ in phase W1)
  development-storage-policy.md
HTML_Mockups/                design reference mock-ups (some show removed features)
```

## 2. By chapter

| Chapter | Active documents | Reference documents |
|---|---|---|
| 1. Complete, test and validate | [roadmap/chapter-1](roadmap/chapter-1-complete-test-validate.md), [Windows phases](roadmap/chapter-1a-windows-phases.md), [Mac phases](roadmap/chapter-1b-mac-phases.md), [testing/](testing), [masterplan.md](implementation/masterplan.md) areas 1 to 17 and 20, [feature-status.md](implementation/feature-status.md), [development-storage-policy.md](development-storage-policy.md), ADR-0027 to 0030 | Specifications, engineering documents 11 to 26, 30 to 39, ADRs |
| 2. Website | [roadmap/chapter-2](roadmap/chapter-2-website.md), [writing standard](engineering/35-writing-standard.md) | [website/README.md](website/README.md), [website/prompts/](website/prompts) (method only), [store/privacy-policy.md](store/privacy-policy.md) |
| 3. Commercial distribution | [roadmap/chapter-3](roadmap/chapter-3-commercial-distribution.md), masterplan areas 18 to 26 | [release/launch-readiness-register.md](release/launch-readiness-register.md), [store/](store), engineering documents [24](engineering/24-windows-packaging-updater.md), [27](engineering/27-microsoft-store.md), [36](engineering/36-entitlement-and-trial.md), [38](engineering/38-mac-app-store-trial.md), ADR-0017, 0018, 0022, 0023 |

Chapters 2 and 3 have no implementation plan yet by decision; each is written when the chapter before it closes.

## 3. By kind of document

| Kind | Documents | Rule |
|---|---|---|
| Product requirements | [product/vision-and-scope.md](product/vision-and-scope.md) and the specifications below | The vision page wins on audience, chapters and order of work |
| Specifications | PRD, FSD, UX/UI, Domain/Data, Revision 1, Import/Export, Offline, Security, AI, Runtime, ESD ([list](specs/README.md)) | Sections marked SUPERSEDED are history; a "Current decisions" note at the top of a specification lists what overrides it |
| Architecture decisions | [adr/](adr) with the [register](adr/README.md) | A new decision is a new ADR and a register row; the old text is marked superseded, not deleted |
| Architecture reference | [architecture/README.md](architecture/README.md), [engineering/](engineering) | Engineering documents implement behaviour and never redefine it |
| Plans | [roadmap/](roadmap), [masterplan.md](implementation/masterplan.md) | The roadmap says what to do and in what order; the build plan keeps the checklist and its evidence |
| Implementation status | [feature-status.md](implementation/feature-status.md), [progress-log.md](implementation/progress-log.md) | A status moves up only with evidence at a named proof level |
| Test evidence | [release/evidence/](release/evidence), [audits/](audits) | What was true on a date, not what must be |
| Operating procedures | [development-storage-policy.md](development-storage-policy.md), [testing/](testing), [engineering/39-packages-restore-drill.md](engineering/39-packages-restore-drill.md) | Rules for people and agents |
| History | [archive/](archive) (index in [archive/README.md](archive/README.md)), [implementation/reconciliation-log.md](implementation/reconciliation-log.md) | Never implement from history |

## 4. Which document wins

Highest first. A lower item never silently overrides a higher one.

1. **Protected invariants**: [CONTRIBUTING.md](../CONTRIBUTING.md) section 2 (architecture, security, data safety).
2. **Accepted ADRs** ([register](adr/README.md)). An Accepted ADR is the recorded decision for the question it answers,
   including where it amends a specification. A newer ADR wins over an older one on the same question. Proposed ADRs change
   nothing until the owner accepts them.
3. **Owner statements recorded in** [owner-decisions.md](implementation/owner-decisions.md) and
   [product/vision-and-scope.md](product/vision-and-scope.md).
4. **Specifications** in [specs/](specs), in this order: PRD, FSD, UX/UI, Domain/Data; then the specialised specs. The
   Screenplay and Production Depth Specification (Revision 1) is the newest and wins over the first four for the areas it
   lists, except where an Accepted ADR says otherwise.
5. **Engineering documents** in [engineering/](engineering).
6. **Execution control**: [roadmap/](roadmap), [masterplan.md](implementation/masterplan.md),
   [feature-status.md](implementation/feature-status.md), [launch readiness register](release/launch-readiness-register.md).
7. **Evidence**: [audits/](audits) and [release/evidence/](release/evidence).
8. **History**: [archive/](archive), [website/prompts/](website/prompts), `HTML_Mockups/`.

[README.md](../README.md) is the release-state declaration (what is usable today), written from the feature status file.

## 5. Current decisions at a glance

Full table with replaced decisions: [adr/README.md](adr/README.md). Owner answers: [owner-decisions.md](implementation/owner-decisions.md).

- **Audience:** one application for writers, screenwriters and production teams ([vision](product/vision-and-scope.md)).
- **Chapters:** 1 complete, test and validate (Windows track, then Mac track), 2 website, 3 store distribution, in that
  strict order ([ADR-0030](adr/ADR-0030-verification-ladder-and-release-flow.md)).
- **Architecture:** one Rust core; Tauri and React on Windows; a SwiftUI shell on the Mac through UniFFI
  ([ADR-0027](adr/ADR-0027-rust-core-two-ui-shells.md)); SQLite per project, UUIDv7 identities, one mutation pipeline, undo,
  recovery (ADR-0001 to 0005, 0011).
- **AI:** local only; measured first, two model tiers, verifier-guided loop, memory, English release bar, eleven further
  languages ([ADR-0028](adr/ADR-0028-ai-quality-program-and-harness.md)); Vulkan graphics-card path with a first-run probe
  ([ADR-0029](adr/ADR-0029-gpu-acceleration.md)); the runtime ships in the package and the models download after install
  ([ADR-0026](adr/ADR-0026-offline-ai-download-after-install.md)); human-only Change Sets, crash-safe apply (ADR-0013 as
  amended, ADR-0019).
- **Collaboration:** file-based Exchange, Review and Response packages only (ADR-0007).
- **Platforms:** Windows 11 and Apple silicon only (ADR-0017). The Intel MacBook is for development.
- **Business model (target, not final):** paid with a 7-day free trial; stores run the trial; internal builds have no trial,
  price or lock (ADR-0018, 0022, 0023, 0024).
- **Internal builds:** unsigned `.exe` and `.dmg` behind a team password of at least 8 characters, tested by the team before
  the next stage (ADR-0024, ADR-0030).
- **Nothing sent automatically** (ADR-0021).

## 6. The specification stack

| Question | Document |
|---|---|
| What are we building and why | [PRD](specs/OpenFrame_Studio_Mega_PRD_Aligned_Updated.md) |
| How each feature behaves | [Functional Specification](specs/OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md) |
| How each screen behaves | [UX/UI Specification](specs/OpenFrame_Studio_UX_UI_Specification_Updated.md) |
| What objects exist | [Domain and Data Specification](specs/OpenFrame_Studio_Domain_Data_Specification_Updated.md) |
| Screenplay depth, production depth, themes (newest) | [Revision 1](specs/OpenFrame_Studio_Screenplay_and_Production_Depth_Specification.md) |
| Offline and packages (LAN removed) | [Offline and Collaboration](specs/OpenFrame_Studio_Offline_Collaboration_Specification.md) |
| Formats | [Import and Export](specs/OpenFrame_Studio_Import_Export_Specification.md) |
| What AI may do | [AI spec](specs/OpenFrame_Studio_AI_Specification_Updated.md), [agent behaviour](specs/openframe_ai_agentic_rag_spec_prompt.md), [runtime](specs/OpenFrame_Local_AI_Runtime_Model_Management_Specification.md) |
| What leaves the computer | [Security, Privacy, Data Ownership](specs/OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md) |
| How engineering builds it | [ESD](specs/OpenFrame_ESD_Technical_Architecture.md), [Engineering Package Index](specs/OpenFrame_Studio_Engineering_Package_Index.md), [engineering/](engineering) |

## 7. Engineering documents

| No. | Document |
|---|---|
| 11 to 14, 16, 17, 20 to 26 | Database, file format, command architecture, IPC, search, autosave and recovery, security threat model, performance budgets, QA strategy, release and migration, Windows packaging, CI/CD, observability |
| 18 | Retrieval intelligence index (AI) |
| 27 | Microsoft Store distribution (Chapter 3) |
| 28, 29, 31 | Archived in phase W1 (`archive/2026-10-05-*`); their risk levels and feature contract now live in CONTRIBUTING.md section 8 |
| 30 | Appearance and themes |
| 32, 33 | Screenplay engine contract, production depth contract |
| 34 | UX laws checklist |
| 35 | Writing standard: no em dashes, no AI-sounding text, banned phrases, how the check works |
| 36 | [Entitlement and trial](engineering/36-entitlement-and-trial.md) (Chapter 3) |
| 37 | [Platform boundary](engineering/37-platform-boundary.md): what differs between Windows and macOS |
| 38 | [Mac App Store trial](engineering/38-mac-app-store-trial.md) (Chapter 3) |
| 39 | [Packages restore drill](engineering/39-packages-restore-drill.md) |
| ai-tool-coverage | Generated inventory of operations and AI tools (never edit by hand) |
| AGENT_BRIEF, SECURITY_REVIEW_2026-09-30 | Archived in phase W1 (`archive/2026-09-30-agent-brief.md`, `archive/2026-09-30-security-review.md`) |

## 8. Keeping documents in step (mandatory)

When a change alters behaviour it must also update, in the same change:

| If you change | Update |
|---|---|
| A feature's state | Feature status row (with its proof level); build plan box with evidence; progress log |
| A registered operation | `engineering/ai-tool-coverage.md` (regenerate), AI decision in the change |
| A schema | A new numbered migration (never edit a frozen one) recorded with `node scripts/check-migrations.mjs --record`, `engineering/11-physical-database.md`, `engineering/23-release-migration.md` version table |
| IPC or DTOs | Regenerated bindings (TypeScript and, from phase M2, Swift) |
| User-visible behaviour | The spec section, the manual source, in-app help, the Store listing draft if it is a listed feature, and the README status table |
| A decision | A new ADR **and** a row in [adr/README.md](adr/README.md); mark the replaced text superseded, never delete it |
| A chapter plan item | The chapter page and the matching build plan box |
| Release or Store setup | `engineering/27-microsoft-store.md`, `release/launch-readiness-register.md` |
| README status claims | The feature status file first; README follows it |
| A feature's screen or tests | The "How a user reaches it" and "Tests" columns of the feature status file (checked by `scripts/check-feature-status.mjs`) |
| An architecture exception (raw SQLite write, `unsafe`, listener, process, IPC command, asset scope) | The reviewed allow-list and reason in `scripts/architecture-lint.mjs`; a security review |
| A moved or renamed document | Links (run the link check), this map, [archive/README.md](archive/README.md) if it is history, [implementation/reconciliation-log.md](implementation/reconciliation-log.md) |
| User-visible text or a new document | [engineering/35-writing-standard.md](engineering/35-writing-standard.md); `npm run check:writing` |
| A finished feature | Delete the dead code and mark replaced documents superseded in the same change |

Rules: one canonical home per fact; link instead of copying; mark a stale statement superseded instead of leaving two
answers; never claim a state the evidence does not support (designed, implemented, tested, benchmarked, packaged,
certified and released are different).

**No conflicting statements.** When a decision changes, the same change searches the current documents for the old
statement and marks each hit superseded with a pointer to the new ADR (a "Current decisions" note at the top of a large
document is acceptable when the hits are many). The 2026-10-15 sweep and its result are in the
[reconciliation log](implementation/reconciliation-log.md).

## 9. Supplied-material integration

How supplied audits, plans, checklists, specifications, prompts and notes were integrated, every conflict found, and the
old-to-new path map and old-code to new-name tables: [implementation/reconciliation-log.md](implementation/reconciliation-log.md).
