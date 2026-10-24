# 03: Reconciliation log

> Created 2026-10-05. Records (1) every conflict found between the supplied material, the existing documents and the
> code, with its resolution or owner; (2) where each supplied document's content now lives, so nothing is lost;
> (3) stale statements corrected. Evidence tags: **[V]** verified in the repository on 2026-10-05; **[S]** stated by a
> supplied document and not independently verified.
> Nothing was built or run. The M0 baseline run turns [S] items into evidence.

## 1. Conflicts and their resolution

| # | Conflict | Evidence | Resolution | Owner |
|---|---|---|---|---|
| C1 | Business model: free/open-source (repo) vs commercial with 7-day no-card trial (launch register). | README, `LICENSE-PENDING.md`, Engineering Index vs register COMM-001..006 | ADR-0018 (Proposed). Commerce rows are conditional in the masterplan. | Owner D1 |
| C2 | Windows support: Windows 11 vs Windows 10 1809+. | README, DEVELOPMENT vs `MinVersion 10.0.17763.0` in `27-microsoft-store.md` and `listing.md` [V] | ADR-0017 recommends Windows 11. Manifest and listing change with the decision. | Owner D3 |
| C3 | macOS: "Windows 11 first, macOS-ready" vs "equal first-class". | Engineering Index vs doc 28 | ADR-0017: architecture rules now, macOS release as M16, outside the Store gate. | Owner D4 |
| C4 | Light theme only vs Light/Dark/System plus 15 themes. | ADR-0012 §7 [V] vs spec §47A | ADR-0014 (Proposed). Docs 30 holds the engineering spec. | Owner D6 |
| C5 | No stored scene numbers vs imported, alphanumeric, frozen production numbers. | ADR-0011 item 3 [V] vs spec §12 | ADR-0015 (Proposed). Closes the ADR-0012 open question. | Owner D7 |
| C6 | PDF Latin-1 only vs P0 Indic-script round trip including PDF. | ADR-0008 [V] vs spec §19 | ADR-0016 (Proposed): embedded fonts and shaping. | Owner D8 |
| C7 | Telemetry: none (README) vs opt-in diagnostics (Engineering Index) vs analytics and crash aggregation (register). | [V] three documents | ADR-0021 (Proposed): nothing automatic; user-initiated diagnostics. | Owner D5 |
| C8 | Model runtime downloaded after install vs bundled for the Store. | `27-microsoft-store.md` §5.4; audit §12, §19 | ADR-0020 (Proposed). | Owner D9 |
| C9 | AI Change Set apply is not crash-atomic. | `change_set.rs` has no journal or recovery [V]; audit §10 | ADR-0019 (Proposed); milestone M1. | Owner D10 |
| C10 | Migration policy: "pre-release migration files may still be edited" (doc 23) vs "never edit a migration on main" (completion plan). | doc 23 §4 vs plan §10 | New rule: files 0001 to 0010 are **frozen at the first external build** (internal RC or beta). Until then edits are allowed but discouraged. Every new schema change is a new numbered file. Recorded in doc 23. | Engineering |
| C11 | Schedule model: plan proposes a new unified `ScheduleItem` type; the repo already has `schedule_marker` with Meal, Travel, Company Move, Custom plus time and duration. | `0007_schedule.sql` [V] | **Extend, do not replace.** Add day timeline columns and strip start time. A unified item table would rewrite working code and violates the plan's own "no rewrite" law. | Engineering |
| C12 | Budget: spec wants Planned and Actual; repo has one `amount`. | `budget_line` [V] | New migration; existing `amount` becomes Planned; add Actual. | Engineering |
| C13 | Templates: repo has 7 built-ins; spec recommends 8 with different members. | `notes/templates.rs` [V] | Owner decision D12. | Owner D12 |
| C14 | Scene card has only `short_description`; spec wants a long description. | `0003_story.sql` [V] | Additive column. | Engineering |
| C15 | Screenplay element types are a fixed CHECK list of 8; custom elements and page breaks need more. | `0004_screenplay.sql` [V] | SQLite cannot alter a CHECK in place: table-rebuild migration with the id/rev/timestamps preserved (doc 23 §4 rule 3). | Engineering |
| C16 | Audit says LAN residue remains in code (`Capability::HostSession`, `ActorOrigin::Lan`, "local-network collaboration" message). | No match in `crates/` for those symbols; `ActorOrigin` has Local, Exchange, Ai; the network-folder message already points to copy or packages [V] | **Already resolved in code.** Remaining LAN text is in specification and manual files (321 mentions in 35 Markdown files). Banners now, body rewrite tracked as DOC-003. ADR-0007 consequences note updated. | Docs |
| C17 | Audit says `DEVELOPMENT.md` claims later crates do not exist. | `DEVELOPMENT.md` §5 text; crates `openframe-search`, `-import-export`, `-ai` exist [V] | Corrected in `DEVELOPMENT.md`. | Docs |
| C18 | Doc 23 says project schema version 9. | 10 files in `migrations/project/` [V] | Corrected to 10. | Docs |
| C19 | `LICENSE-PENDING.md` and older notes mention a Qwen GGUF. | `LICENSE-PENDING.md` L26 [V]; ADR-0013 and Local AI spec say Gemma 3 1B and BGE small | Corrected in `LICENSE-PENDING.md`. | Docs |
| C20 | Engineering Index lists documents `10_` to `30_` by names that do not exist; real files are `docs/engineering/NN-*.md` and the specifications in `docs/specs/`. | [V] | Index rewritten with real paths. | Docs |
| C21 | Engineering Index Phase 6 still lists LAN host, discovery, soft locks. | [V] | Replaced with file-package collaboration. | Docs |
| C22 | Completion plan says "no screenplay pagination; build once". The repo already has a PDF layout engine (`layout.rs`, 868 lines) with MORE/CONT'D. | [V] | Plan treats it as the starting point; editor parity is verified in M2. | Engineering |
| C23 | Store listing and privacy policy describe features the README marks "In development". | `docs/store/listing.md` vs README [V] | Listing carries a gating banner: no claim may be submitted until its ledger row is S5. | Release |
| C24 | The supplied checklist and the completion plan use different phase numbers (0 to 13 vs 0 to 11), the OS plan uses A to F, the Store audit uses P0/P1/P2. | [V] | Single milestone list M0 to M16 in the masterplan with a crosswalk (§3). | Docs |
| C25 | AI "agentic spec" file `openframe_ai_agentic_rag_spec_prompt.md` sits in the root as a prompt. | [V] | Kept in place because code and docs cite its section numbers; classified as the implemented AI behaviour spec in `docs/README.md`. | Docs |
| C26 | The user manual (`.docx`, `.pdf`) still contains the LAN figures 164 to 168. | **Verified 2026-10-05 [V]:** text extracted from the `.docx` has 181 figures and shows "Starting a local collaboration session (host)" (164) and "Joining a session (participant)" (165) | Regeneration planned (DOC-007). Source of truth for the manual to be decided in M10. | Docs |

## 2. Where each supplied document lives now

| Supplied document | Now | Treatment |
|---|---|---|
| `OpenFrame_New_Spec_Revised.md` | `OpenFrame_Studio_Screenplay_and_Production_Depth_Specification.md` (repo root) | Promoted to a normative specification (Revision 1), header added, content unchanged. §47A kept with its numbering. |
| `OpenFrame_End_to_End_Completion_Implementation_Plan_2026-10-03.md` | `docs/archive/2026-10-03-end-to-end-completion-plan-source.md` | Archived with a banner. Its strategy, laws, completion states, AI integration pipeline, PR checklist and agent protocol are absorbed into `00-masterplan.md`, `CONTRIBUTING.md` and doc 31. |
| `OpenFrame_Manual_vs_GitHub_Audit_Updated_2026-10-03.md` | `docs/audits/2026-10-03-manual-vs-repository-audit.md` | Evidence snapshot. 181-figure table stays here; the ledger references figure ranges. |
| `OpenFrame_Production_Microsoft_Store_Readiness_Audit_2026-10-02.md` | `docs/audits/2026-10-02-production-and-store-readiness-audit.md` | Evidence snapshot. Every P0/P1/P2 item became a masterplan or ledger row (see §4). |
| `OpenFrame_Master_Product_Launch_Operations_Readiness_Checklist_REVISED.md` | `docs/release/launch-readiness-register.md` | Promoted as the canonical operations register; additions from the intake notes appended (Appendix F). |
| `OpenFrame_Optimal_HLD_LLD_Desktop_App_2026-10-03.md` | `docs/engineering/29-desktop-hld-lld.md` | Promoted as target architecture; header explains overlap with the ESD. |
| `OpenFrame_Strong_OS_Compatibility_and_Security_Windows_macOS_2026-10-03_REVISED.md` | `docs/engineering/28-cross-platform-compatibility-security.md` | Promoted; scoped by ADR-0017. |
| `OpenFrame_Vibe_Coding_Gaps_and_Coverage_2026-10-03.md` | `docs/engineering/31-ai-assisted-development-controls.md` | Promoted; CONTRIBUTING points to it. |
| `claude_premium_website_design_motion_prompt.md` | `docs/website/prompts/premium-website-design-motion-prompt.md` | Kept as a prompt. Used by `docs/website/README.md`. Tool names in it are unverified. |
| `claude_production_readiness_anti_ai_slop_prompt.md` | `docs/website/prompts/production-readiness-anti-ai-slop-prompt.md` | Same. |
| `info.txt` (Store certification tips, brand checklist, 40 extra items) | `docs/archive/notes/store-certification-and-launch-gap-notes.md` | Content absorbed into `27-microsoft-store.md` §12 and register Appendix F. |
| `All required docs.txt` (documentation stack guidance) | `docs/archive/notes/documentation-stack-guidance.md` | Absorbed into `docs/README.md` documentation map. |

## 3. Phase crosswalk

| Masterplan | Completion plan | Launch register Appendix B | OS plan | Store audit |
|---|---|---|---|---|
| M0 | Phase 0 | Phase 0 | | P0 items 2, 3, 4 |
| M1 | Phase 0 and 11 (hardening) | Phase 0, 8 | | P0 12; P1 15 to 17 |
| M2 | Phase 1, 8 | Phase 1 | | |
| M3 | Phase 2 | Phase 2 | | |
| M4 | Phase 3 | Phase 3 | | |
| M5 | Phase 4 | Phase 4 | | |
| M6 | Phase 5 | Phase 5 | | |
| M7 | Phase 6, 7 | Phase 6 | | |
| M8 | Phase 9 | Phase 7 | | |
| M9 | Phase 10 | Phase 8 | | P0 6, 7; P1 13, 14 |
| M10 | Phase 11 | Phase 8, 12 | | P1 21, 24 |
| M11 | Phase 11 | Phase 9, 10 | | P0 8 to 11; P1 20, 22 |
| M12 | | Phase 11 | | P0 5 |
| M13 | | Phase 12 | | |
| M14 | | Phase 12, 13 | | |
| M15 | | Phase 13 | | |
| M16 | | Phase 9, 10 (macOS, direct) | Phases A to F | P2 25, 26 |

## 4. Store audit findings mapped

| Audit item | Where it is tracked |
|---|---|
| Choose first Store scope | D2, M0 |
| `cargo deny` failing | PLAT-003, M0 |
| Licence selection | REL-001, M0 |
| Partner Center identity | REL-002, M0 |
| Privacy policy final | REL-006, M12 |
| AI output reporting | AI-010, M9 |
| Production AI distribution trust | AI-012, M9 |
| Supported Windows versions | PLAT-001, M0 and M11 |
| RC MSIX from real pipeline | REL-003, M11 |
| WACK | REL-004, M11 |
| Installed-package E2E on clean machines | REL-005, M11 |
| Change Set interrupted-apply recovery | AI-005, M1 |
| Hardware benchmarks | AI-004, M9 |
| Retrieval evaluation | AI-003, M9 |
| Desktop E2E required | PLAT-002, M1 |
| Forced-kill and soak tests | CORE-003, CORE-004, M1 |
| Bundle runtime in MSIX | AI-011, M9 |
| Diagnostics export | CORE-015, M1 |
| Accessibility and high-DPI pass | CORE-016, CORE-017, M10 |
| Listing accuracy | REL-007, M13 |

## 5. Items from the intake notes (`info.txt`) not in the supplied register

Added to the register as Appendix F: Store certification pre-flight rows (WACK, clean sandbox, privacy URL,
IARC rating for mature screenplay content, Notes for Certification, manifest capabilities), real-filmmaker beta,
launch trailer, founder story, creator and community outreach, 30-day content bank, status and outage
communication, customer feedback system, localization architecture, public documentation, versioning strategy,
support SLA, funnel analytics (reconciled with ADR-0021), post-launch incident response.


## 6. Documentation restructure 2026-10-05 (nothing deleted)

Reason: the repository root held 13 loose specification files plus the manual. They moved with `git mv` (history kept) and names are unchanged. Links were rewritten and checked (0 broken). No code or script referenced them by name.

| Old path (repository root) | New path |
|---|---|
| `OpenFrame_Studio_Mega_PRD_Aligned_Updated.md` | `docs/specs/OpenFrame_Studio_Mega_PRD_Aligned_Updated.md` |
| `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md` | `docs/specs/OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md` |
| `OpenFrame_Studio_UX_UI_Specification_Updated.md` | `docs/specs/OpenFrame_Studio_UX_UI_Specification_Updated.md` |
| `OpenFrame_Studio_Domain_Data_Specification_Updated.md` | `docs/specs/OpenFrame_Studio_Domain_Data_Specification_Updated.md` |
| `OpenFrame_Studio_Screenplay_and_Production_Depth_Specification.md` | `docs/specs/OpenFrame_Studio_Screenplay_and_Production_Depth_Specification.md` |
| `OpenFrame_Studio_Offline_Collaboration_Specification.md` | `docs/specs/OpenFrame_Studio_Offline_Collaboration_Specification.md` |
| `OpenFrame_Studio_Import_Export_Specification.md` | `docs/specs/OpenFrame_Studio_Import_Export_Specification.md` |
| `OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md` | `docs/specs/OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md` |
| `OpenFrame_Studio_AI_Specification_Updated.md` | `docs/specs/OpenFrame_Studio_AI_Specification_Updated.md` |
| `OpenFrame_Local_AI_Runtime_Model_Management_Specification.md` | `docs/specs/OpenFrame_Local_AI_Runtime_Model_Management_Specification.md` |
| `openframe_ai_agentic_rag_spec_prompt.md` | `docs/specs/openframe_ai_agentic_rag_spec_prompt.md` |
| `OpenFrame_ESD_Technical_Architecture.md` | `docs/specs/OpenFrame_ESD_Technical_Architecture.md` |
| `OpenFrame_Studio_Engineering_Package_Index.md` | `docs/specs/OpenFrame_Studio_Engineering_Package_Index.md` |
| `OpenFrame_Studio_User_Manual.docx` | `docs/manual/OpenFrame_Studio_User_Manual.docx` |
| `OpenFrame_Studio_User_Manual.pdf` | `docs/manual/OpenFrame_Studio_User_Manual.pdf` |

Also created: `docs/adr/README.md` (decision register: active, superseded, proposed), `docs/specs/README.md` (status of each specification), `docs/spec-digest/README.md` (digests are derived and partly stale).

Older decisions: 57 section headings carrying live-LAN content were marked **SUPERSEDED by ADR-0007** in place (PRD 2, FSD 13, UX 3, Domain 1, Offline/Collaboration 16, Security 22). Text unchanged. Nothing was removed. Other superseded or amended decisions are listed in `docs/adr/README.md` section 2.

Remaining at the root by design: README, CONTRIBUTING, DEVELOPMENT, CLAUDE, LICENSE-PENDING, THIRD_PARTY_LICENSES (the last is written by `scripts/license-inventory.mjs`).

Masterplan recut: every box now has a wave tag (W1 to W5 and the owner track WO); W1 is 56 of 224 boxes (25.0%); box M0.28 (Windows security baseline audit) and section 1a (security standard) were added.

2026-10-05 (Wave 1 session 1): the M0.28 security baseline added 10 boxes (M1.21 to M1.28 in W1, M11.15 and M11.16 in W4) and amended M1.18, M9.09, M9.16 and M10.07. Totals are now 234 boxes, W1 64 (27.4%). ADR-0019 and ADR-0021 were accepted by the owner, who delegated the choice to the recorded recommendation. The ledger gained `UI route` and `Tests` columns and per-section `Source:` lines; all [R]/[A] tags were replaced with evidence (M0.19), which corrected 65 states. Earlier ledger statements that were wrong: XFER-001 (unknown FDX constructs are flattened into Action), XFER-008 (the import report lives in `modules/interchange/import.rs`), XFER-010 (no PDF golden bytes exist), SCRIPT-010 (keep-together rules exist), CORE-013 (the quoted "unavailable offline" string does not exist), CORE-014 (only panic and startup lines are redacted), VAULT-004 (research flags are not implemented), BRK-004 and BRK-005 (partially implemented, not missing). Doc 23 also listed the global schema version as 2; it is 3.
