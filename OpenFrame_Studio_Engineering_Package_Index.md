# OpenFrame Studio — Engineering Package Index

    **Product:** OpenFrame Studio  
    **Document class:** Implementation / engineering specification  
    **Baseline:** Windows 11 production architecture, v1  
    **Generated:** 29 September 2026  
    **Purpose:** Define the implementation-document set, authority boundaries, build order, decision register, and readiness gates for production engineering.


## Authority and non-conflict rule

This engineering document implements the approved OpenFrame specifications. It does **not** redefine product behavior.

Implementation precedence is:

1. Product PRD — scope, intent, priority and product boundaries.
2. Functional Specification — observable behavior and acceptance contract.
3. UX/UI Specification — presentation and interaction.
4. Domain & Data Specification — canonical identities, relationships, lifecycle and source-of-truth.
5. AI / Import-Export / Offline-Collaboration / Security specifications — cross-cutting specialized contracts.
6. This engineering document — concrete implementation choices.

If an implementation detail conflicts with an authoritative product rule, the implementation detail must change. Engineering must not “fix” the product by silently changing behavior.


## Locked implementation decisions

These decisions were explicitly approved for the first implementation baseline:

| Decision | Locked choice |
|---|---|
| Initial OS | Windows 11 first; preserve macOS-ready boundaries |
| Desktop shell | Tauri v2 |
| UI | React + TypeScript |
| Core/business logic | Rust owns domain rules; React is presentation/input |
| Local persistence | SQLite + filesystem assets |
| Project UX | One `.openframe` project concept; internal files are implementation detail |
| Offline AI | One-click **Download Offline AI**; OpenFrame installs runtime/model |
| Model selection | Hardware-aware automatic recommended Qwen profile |
| Model distribution | OpenFrame-controlled public model distribution endpoint/CDN |
| AI providers | **Local AI only** for v1; no cloud AI provider integration |
| Accounts | No account required |
| Telemetry | Off by default; optional anonymous diagnostics/crash reports with explicit consent |
| Product model | Free/open-source; exact OSS license identifier must be selected before public release |
| LAN collaboration | Architecture-ready now; ship after stable solo core |
| Concurrent collaboration | Soft locks + optimistic version/conflict handling; no CRDT in v1 |
| Autosave | Continuous autosave + transactional writes + recovery history |
| Encryption | OS security by default; optional encrypted project/package mode is later scope |
| Updates | Notify user; download/install only after explicit approval |
| Migration | Safety backup first, then transactional migration with rollback/recovery path |
| Offline UX | Disable only unavailable online operations; local work continues normally |


## 1. Package purpose

The existing OpenFrame documents already define the product in depth. This package fills the engineering gap between those behavioral specifications and a production codebase. It deliberately does not regenerate or replace the PRD, FSD, UX/UI, Domain/Data, AI, Import/Export, Offline/Collaboration, Security/Privacy, or User Manual.

## 2. Existing authoritative inputs

- `OpenFrame_Studio_Mega_PRD_Aligned_Updated(3).md`
- `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md`
- `OpenFrame_Studio_UX_UI_Specification_Updated.md`
- `OpenFrame_Studio_Domain_Data_Specification_Updated.md`
- `OpenFrame_Studio_AI_Specification_Updated.md`
- `OpenFrame_Studio_Import_Export_Specification.md`
- `OpenFrame_Studio_Offline_Collaboration_Specification.md`
- `OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md`
- OpenFrame Studio User Manual / interface mock-up reference

## 3. Generated implementation package

| No. | File | Owns |
|---:|---|---|
| 10 | `10_OpenFrame_ESD_Technical_Architecture.md` | System architecture and technology choices |
| 11 | `11_OpenFrame_Physical_Database_SQLite_Schema_Specification.md` | Physical persistence model and SQLite rules |
| 12 | `12_OpenFrame_Local_Project_File_Format_Specification.md` | `.openframe`, assets, packages and manifests |
| 13 | `13_OpenFrame_Rust_Domain_Command_Architecture_Specification.md` | Rust domain/application architecture and command model |
| 14 | `14_OpenFrame_Tauri_IPC_Frontend_Backend_Contract.md` | React ↔ Rust boundary |
| 15 | `15_OpenFrame_Local_AI_Runtime_Model_Management_Specification.md` | Local inference, model manager and downloads |
| 16 | `16_OpenFrame_Search_Retrieval_Architecture.md` | Exact search, FTS and optional semantic retrieval |
| 17 | `17_OpenFrame_Autosave_Undo_Recovery_Backup_Engineering_Specification.md` | Save/recovery durability |
| 18 | `18_OpenFrame_LAN_Networking_Conflict_Protocol_Specification.md` | LAN sessions, soft locks and conflicts |
| 19 | `19_OpenFrame_Import_Export_Implementation_Architecture.md` | Import/export execution architecture |
| 20 | `20_OpenFrame_Security_Threat_Model_Secure_Storage_Implementation.md` | Threat model and implementation controls |
| 21 | `21_OpenFrame_Performance_Resource_Budget_Specification.md` | Latency, RAM, disk and startup budgets |
| 22 | `22_OpenFrame_QA_Acceptance_Test_Strategy.md` | Test architecture, traceability and release gates |
| 23 | `23_OpenFrame_Release_Migration_Compatibility_Specification.md` | Schema/project compatibility and migrations |
| 24 | `24_OpenFrame_Windows_Packaging_Signing_Updater_Specification.md` | Windows installer/signing/updater |
| 25 | `25_OpenFrame_CI_CD_Build_Reproducibility_Specification.md` | CI, artifacts, SBOM and reproducibility |
| 26 | `26_OpenFrame_Observability_Logs_Crash_Reporting_Specification.md` | Local logs and opt-in diagnostics |
| 27 | `27_README.md` | Repository-facing introduction |
| 28 | `28_DEVELOPMENT.md` | Local development/build workflow |
| 29 | `29_CONTRIBUTING.md` | Contribution and review contract |
| 30 | `30_Architecture_Decision_Records.md` | Architecture decisions and rejected alternatives |

## 4. Implementation phases

### Phase 0 — Repository foundation
- Rust workspace and Tauri shell.
- React/Vite/TypeScript UI shell.
- Shared generated IPC types.
- SQLite migration harness.
- error taxonomy, structured logging and test harness.
- Windows CI build.

### Phase 1 — Durable project core
- project create/open/close.
- `.openframe` layout.
- database transaction layer.
- continuous autosave.
- command/undo/recovery framework.
- project files and external reference handling.
- backups and crash recovery.

### Phase 2 — Creative core
- Idea Vault.
- Story Board, Acts, Sequences, Beats and Scene Cards.
- Characters and Story Timeline.
- global search/indexing.

### Phase 3 — Screenplay
- structured screenplay editor.
- screenplay scene/element model.
- drafts/history/compare/review.
- lock/revision behavior.
- import/export baseline.

### Phase 4 — Production
- production source.
- breakdown/catalog.
- locations/cast/crew.
- moodboards/storyboards/shots.
- shooting schedule.
- call sheets/reports/sides.

### Phase 5 — Local AI
- model manager.
- one-click model download.
- hardware profile selection.
- local `llama.cpp` sidecar.
- deterministic tool registry and Change Set workflow.
- project-aware retrieval.

### Phase 6 — Collaboration
- exchange packages first.
- LAN host/discovery/session.
- roles and scope.
- soft locks.
- optimistic concurrency and conflict review.

### Phase 7 — Production hardening
- migration compatibility.
- installer/signing/updater.
- performance budgets.
- security review.
- accessibility.
- complete regression suite and release candidate gates.

## 5. Non-negotiable engineering invariants

1. React never writes SQLite directly.
2. AI never writes SQLite or arbitrary project files directly.
3. Every persistent project mutation is a Rust application command.
4. Every command that mutates canonical state executes in a transaction.
5. Every persistent object uses stable opaque identity; display numbers are not identity.
6. Filesystem assets are referenced through project-managed records and validated paths.
7. Project import is validate → preview → Change Set → apply.
8. Historical snapshots and issued exports are immutable snapshots.
9. LAN participants never open the host SQLite file over a network share.
10. Core app operation does not depend on an account, internet, cloud database or cloud AI.
11. Local AI failure never blocks core editing.
12. The application must preserve recoverable state on write failure wherever technically possible.
13. Model/runtime downloads are not trusted until integrity verification succeeds.
14. Private Notes are a distinct authorization boundary.
15. New implementation work requires tests and traceability to approved behavior.

## 6. Definition of engineering-ready

A feature is engineering-ready only when:
- authoritative behavior is identifiable;
- canonical domain objects are known;
- command inputs/outputs and permission requirements are defined;
- persistence changes are known;
- undo/recovery semantics are known;
- error states are known;
- QA acceptance cases are known;
- no unresolved product decision is silently being made in code.

## 7. Explicit open decisions

The following are deliberately not invented:
- exact open-source license identifier (MIT, Apache-2.0, GPL-family, etc.);
- final public model/CDN hostname;
- final code-signing certificate provider;
- final crash-report collection service, if opt-in remote reporting is enabled;
- final brand typography/assets beyond the approved UX specification.

These do not block architecture implementation, but release automation must fail closed where the value is legally or operationally required.
