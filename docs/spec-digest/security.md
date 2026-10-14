# Security / Privacy / Data Ownership Specification: Implementation Digest

Source: `OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md`. All 3589 lines were read: §0–§69 plus Appendices A–C.
Requirement IDs used here: `SEC-001`…`SEC-060` and `SEC-AC-001`…`SEC-AC-050`. These IDs are local to the spec (App. C).

> **Build override: LOCAL AI ONLY.** The approved build decisions are: managed llama.cpp sidecar with Qwen GGUF; no cloud AI, no BYOK, no Ollama; no accounts; no telemetry by default. Every passage about External AI, providers, provider retention, or external transmission is marked **⚠ SUPERSEDED (LOCAL AI ONLY)**. The spec never mentions API keys or BYOK. The spec's "no mandatory cloud account" and "no project-content telemetry" rules match the approved "no accounts / no telemetry by default" decisions.

Conventions: "Spec:" means near-verbatim wording. **NOT SPECIFIED** means the spec is silent, so treat the point as an open engineering decision and do not invent behavior.

---

## 0. Contract and concepts (§0)

- **Central principle:** **"The filmmaker owns the project; OpenFrame is the workspace."**
- Security cannot be reduced to "protect the cloud account". Because there is no account, the spec protects:
  - local project access
  - user/project permissions
  - private notes/context
  - data exposed through AI
  - document exports
  - Exchange Packages
  - full-project and backup packages
  - LAN session visibility
  - recovery/history
  - imported content
  - activity history
  - external-provider disclosure (⚠ SUPERSEDED)
- The header operating model reads: "optional local AI; **optional external AI** (⚠ SUPERSEDED); optional local-network collaboration; no mandatory OpenFrame cloud".
- Governing rules (§0.2):
  - The PRD owns intent/scope.
  - The FSD owns behavior.
  - UX/UI owns presentation.
  - Domain/Data owns identity, ownership, versioning, and source of truth.
  - AI owns AI access, command, permission, disclosure, and approval.
  - Import/Export owns portability.
  - Offline/Collab owns offline, LAN, presence, and conflict.
  - This spec owns the security, privacy, and ownership interpretation.
  - "No specialist document may create an independent source of truth." "No security control may silently change approved product scope."
- Three distinct concepts (§0.3):
  - **Security:** "Who is allowed to access or mutate this information or state?"
  - **Privacy:** "What information is allowed to be disclosed, transmitted, exported, or shown in this context?"
  - **Data ownership:** "Who controls the project as a user-owned working asset, and what does sharing actually grant?"
  - Examples: an owner can grant Viewer access. An Editor can still be barred from another user's Private Notes. A package recipient receives no project-wide access. An external AI provider (⚠) does not become owner.

## 1. Core principles (§1, normative)

| # | Principle | Rule |
|---|---|---|
| 1.1 | Local ownership | The saved local project is the primary state. OpenFrame must not behave as if the platform owns it |
| 1.2 | No mandatory cloud account | Create, open, edit, save, import, export, backup, recovery, and core work must not depend on an OpenFrame cloud account |
| 1.3 | **No silent transmission** | "Project information must not be transmitted outside the local application merely because a user opened a project, edited a screenplay, or used a normal offline-capable workflow" |
| 1.4 | Explicit external AI disclosure | ⚠ SUPERSEDED (no external AI) |
| 1.5 | Scope-limited disclosure | Only the context required for the operation. ⚠ It is framed for external AI, but the minimization principle still applies to local model context (§9.1) |
| 1.6 | Private-note protection | Private Notes are a "stronger privacy boundary" than ordinary content. They are never surfaced via AI or collaboration merely because they exist in the project |
| 1.7 | Permission inheritance | Effective permission applies to local use, Exchange Packages, LAN, AI, import, and export |
| 1.8 | User-controlled mutation | An NL command does not authorize a mutation. Preview plus explicit acceptance is required |
| 1.9 | No direct AI storage authority | No unrestricted filesystem access. The AI goes through app tools and the canonical domain layer |
| 1.10 | No silent overwrite | Import, exchange reconciliation, conflict resolution, and recovery must not silently destroy state |
| 1.11 | Recoverability | Security controls must not create a second hidden source of truth. Preserve recoverable local state and make it visible |
| 1.12 | **No false security claims** | Never claim encryption, cloud backup, secure sync, provider retention, or any other guarantee unless it is implemented and in the contract |

## 2. Data ownership rules (§2, §40, §43, §62)

- **Project content (§2.1):**
  - Idea Vault, Story Board, screenplay drafts/scenes, comments, **Private Notes**
  - breakdowns, catalog, locations, cast/crew records, moodboards, storyboard panels, shots, schedules, call sheets
  - project notes, tasks, lightweight budget, project files, history/recovery state, metadata
  - "The user's local project file is the primary saved working asset."
- **OpenFrame is not the owner (§2.2).** It "must not present ordinary project data as if it were owned by OpenFrame."
- **Ownership is not unrestricted collaborator access (§2.3).** Sharing does not erase role restrictions, project scope, object/state restrictions, private-note boundaries, screenplay lock, package boundaries, or AI permission boundaries.
- **Export does not transfer ownership (§2.4).** The four export types:
  - Document export = snapshot
  - Exchange Package = portable collaboration/review snapshot
  - Full Project Package = portable working-project transfer
  - Backup Package = recovery copy
  - None of them means "OpenFrame taking ownership".
- **Open-as-copy (§2.5).** A Full Project Package opened as a copy gets a new identity, distinct from the original. The source identity may remain as provenance. The copy must not "accidentally become a second live write path to the original project."
- **"Replace Existing After Backup" (§2.6)** is a deliberate local replacement, not sync. It does not give the source package permanent control. It is subject to the destination user's permissions and the import safety rules.
- **Multi-user (§40):**
  - One logical project can have many permissions.
  - Comments, activity attribution, AI requests, Change Sets, and private notes stay tied to user identities.
  - "Private does not mean invisible to the owner": the owner's Private Notes remain in *their* context.
  - Content deliberately turned into shared content follows normal permissions.
- **Removing a collaborator (§40.5)** changes *future* access only. It does not recall exported PDFs, package files, screenshots, copied text, or other transferred material.
- **Exported copies (§62).** Distinguish "OpenFrame access control" from "Control over copies already exported". Make outgoing scope clear before export and do not promise "impossible revocation".
- **AI provenance (§43).** "AI output is not project authority." Only the final accepted application mutation becomes project state. "The model itself never becomes an alternate project owner or database."
- **Source of truth (§41).** A security feature "must not create a second writable copy" of any canonical source:

| Area | Source of truth |
|---|---|
| Idea Vault | Global/Project Idea Vault |
| Story Board | Acts / Sequences / Beats / Scene Cards |
| Screenplay | Screenplay Draft / Scene / Element |
| Production Source | Selected Draft |
| Breakdown | Breakdown + Catalog references |
| Shot List | Shots |
| Storyboard | Storyboard Panels |
| Shooting Schedule | Shooting Days / Markers |
| Call Sheet | Call Sheet document |

- **Derived data (§42).** Scene numbers, counts, schedule summaries, statistics, and relationship queries are recalculated. "Security evaluation must occur against the underlying accessible source data." A derived summary must not leak data from a broader hidden scope.

## 3. Security scope (§3)

- **Lifecycle:** Create → Edit → Autosave/Manual Save → History/Recovery → Export → Share/Exchange → Import/Reconcile → Collaboration → Archive/Delete/Restore.
- **Processing boundaries:** Local Application, Local AI, External AI (⚠ SUPERSEDED), Local Network Collaboration, Document Export, Exchange Package, Full Project Package, Backup Package. Each boundary preserves effective permissions and privacy rules.

## 4. Data classification (§4)

| Class | Name | Contents | Rule |
|---|---|---|---|
| A | Project content | Story, screenplay, breakdown, production, schedule, call-sheet, notes/tasks, files, approved/historical docs | Follows project permissions + workflow scope |
| B | Private project content | Private Notes | Owner/authorized private context only. Never exposed because the surrounding project is accessible |
| C | Derived/historical state | Snapshots, Automatic History Points, deleted/recoverable state, issued call-sheet snapshots, exported snapshots, exchange snapshots, recovery state | "Still project-sensitive… not automatically less private just because they are historical" |
| D | Collaboration context | Collaboration Session, host/participant identity, granted role, shared scope, presence, temporary locks, conflict state | Operational; not permanent project content |
| E | AI interaction data | AI Request, request text, scope, targets, AI Result, suggested Change Set, provenance, external-processing state (⚠), provider/model reference | "May itself contain project-sensitive content" and follows the user/project privacy boundary |
| F | Local profile/context | Application User: stable identity, display name, roles, local profile, effective permissions | App identity/context, "not an enterprise HR identity system" |

## 5. Identity (§5)

- **Application User fields:** `user_id` (stable local identity), `display_name`, `roles`, `local_profile`, `permissions` (effective, derived from role/context).
- **Stable identity (§5.2).** Identity must not change when the display name changes, and renaming an object keeps its identity. Identity matters for activity attribution, comment authorship, private-note ownership, collaboration participation, change-set responsibility, and review provenance.
- **Authentication is NOT specified (§5.3):**
  - Ordinary local use needs no cloud login. This matches the approved **no accounts** decision.
  - Collaboration must admit only users who have been granted session access.
  - The LAN admission mechanism is "an implementation/security design decision".
  - The spec "does not invent passwords, accounts, identity providers, or token formats".
  - Required outcome: **"An unauthorized participant must not gain the permissions or project scope of an authorized participant."**

## 6. Roles and permissions (§6, §7, §16.4, SEC-004..009)

### 6.1 Canonical role set: exactly these five
| Role | Access meaning (verbatim) |
|---|---|
| **Owner** | Full project control |
| **Editor** | Edit authorized project content |
| **Commenter** | View permitted content and comment/review where allowed |
| **Viewer** | Read-only |
| **Export-only** | Create permitted snapshots/packages without editing project data |

- "These roles are the security model."
- Professional labels (Writer, Director, Producer, Assistant Director, Reviewer, Contributor) are **not** security roles. For example, a person can be a Writer and an Owner, or a Director and an Editor (§6.2).
- **Terminology Lock (App. B):**
  - Owner, not "Administrator"
  - Editor, not "Contributor"
  - Commenter, not "Reviewer"
  - Viewer = "Read-only collaborator only"
  - Export-only, not "Download-only account system"
  - Application User, not "Enterprise account / HR user"
  - Private Note, not "Shared note"
  - Exchange Package, not "Live sync object"
  - Full Project Package, not "Review package"
  - Backup Package, not "Collaboration package"
  - Collaboration Session, not "Permanent shared project"
  - Snapshot, not "Live synchronized copy"
  - Change Set, not "Hidden mutation script"
  - Activity Entry, not "Full telemetry log"
  - Local AI, not "Cloud AI"
  - External AI, not "Local AI"
  - Project owner, not "Platform owner"
- ⚠ The AI spec §19.3 uses "Contributor". Implement it as **Editor**.

### 6.2 Per-role rules (explicit text)
- **Owner (§6.3):**
  - can manage project settings, permissions, and collaboration session ownership
  - can edit permitted content
  - can perform owner-only destructive ops
  - can end the collaboration session
  - can perform supported deletion/restore
  - Owner authority is still "subject to object/state restrictions that the application defines for safety".
- **Editor (§6.4):**
  - may edit authorized content
  - does NOT automatically gain ownership, permission-management authority, unrestricted destructive control, another user's private notes, or the "ability to bypass locked or approved states".
- **Commenter (§6.5):**
  - may view permitted content, add comments, and reply/resolve where allowed
  - "cannot use comments, AI, import, or collaboration mechanisms as a route to edit core screenplay/story/production content when the permission model prohibits it."
- **Viewer (§6.6):**
  - read-only
  - cannot turn a read into a mutation via AI command, import, exchange response, collaboration, script lock interaction, or export workflow.
- **Export-only (§6.7):**
  - can create permitted snapshots/packages without editing
  - "cannot enter a live editing session"
  - "Export-only access is not project ownership."

### 6.3 Per-action permission matrix
Cell key:
- **Y** = the spec explicitly allows it
- **N** = the spec explicitly forbids it
- **Allowed†** = "where allowed / authorized content / permitted"
- **—** = not stated; decide explicitly

| Action | Owner | Editor | Commenter | Viewer | Export-only | Source |
|---|---|---|---|---|---|---|
| View permitted content | Y | Y | Y | Y | — (implied for snapshot creation) | §6.1, §6.5 |
| Edit project content | Y (permitted content) | Allowed† (authorized content only) | N (core screenplay/story/production) | N | N ("without editing project data") | §6.3–6.7 |
| Add comment | — | — | Y | N | — | §6.5 |
| Reply/resolve comment | — | — | Allowed† | N | — | §6.5 |
| Manage project settings | Y | N (no automatic authority) | N | N | N | §6.3, §6.4 |
| Manage permissions / change roles | Y | N | N | N | N | §6.3, §6.4 |
| Start/own/end LAN session | Y ("manage collaboration session ownership", "end the collaboration session"; §56.5 "Owner starts session") | — | — | — | N (cannot enter live editing session) | §6.3, §56.5 |
| Join live editing session | Y | Y | — | — | **N** | §6.7, SEC-009 |
| Owner-only destructive ops | Y | N ("unrestricted destructive control" not granted) | N | N | N | §6.3–6.4 |
| Supported delete/restore | Y | — | N | N | N | §6.3 |
| Permanent deletion | Y (deliberate confirmation) | — | N | N | N | §6.3, §14.4 |
| Bypass locked/approved states | N (subject to state restrictions) | N | N | N | N | §6.3, §6.4, §27 |
| Unlock a draft | — (visibility ≠ permission) | — | N | N | N | §7.3 |
| Read another user's Private Notes | N | N | N | N | N | §6.4, §8 |
| Read own Private Notes | Y | Y | Y | Y | — | §8.1 |
| Export document snapshot | Y (within scope) | — | — | N via export workflow as mutation route; export itself is not stated | Y (permitted snapshots) | §6.6, §6.7 |
| Create Exchange/Review Package | Y | — | — | — | Y (permitted packages) | §6.7 |
| Import into project (mutating) | Y | Allowed† | N (bypass route) | N | N | §6.5–6.7 |
| Apply AI Change Set | Y | Allowed† | N (core content) | N | N | §7.3, §16.4 |
| Ask AI read/compute/navigate | Y | Y | Y | Y | — | AI spec §5 |

"Visibility ≠ permission" (§7.3): being able to see an object does not imply the user may edit, delete, export all related data, share, change permissions, unlock a draft, or apply an AI Change Set.

### 6.4 Permission evaluation (§7)
- **Inputs (§7.1):** User + Project + Role + Shared Scope/Package Scope + Target Object + Object State + Operation.
- **Before any mutation (§7.2):**
  1. requesting user
  2. current project
  3. requested scope
  4. target object(s)
  5. current object state
  6. effective role/permission
  7. whether the operation is allowed
  8. whether an extra confirmation/approval gate is required
- **Denial (§7.4) must:**
  - not mutate data
  - explain the blocked operation "at the appropriate UX level"
  - "not silently downgrade into a different write operation"
  - "not reveal protected private information merely to explain the denial"
  - If a protected object's *existence* is private, do not disclose it.
- **Approval ≠ permission (§26.3, SEC-056).** "Approval does not override permission restrictions." Both must be valid.

## 7. Private Notes (§8, §10.2, §13.5, §33.2, §34.1, §36D)

- **Domain fields:** `private_note_id`, `owner_user_id`, optional target, body, timestamps. Rule: **"Private Notes belong to their owner/authorized private context."**
- **They stay private during:**
  - Solo Local
  - Exchange Package preparation and import
  - LAN collaboration
  - AI interaction
  - export
  - review
  - offline use
- **Not a hidden shared database (§8.3).** Selecting the whole project for collaboration does not share Private Notes. Only a deliberate normal-app transformation makes them shared content.
- **AI (§8.4):** own notes are usable when the user is the owner/authorized context, the request is permitted, and the context is intentionally included. The AI must never expose another collaborator's notes.
- **Refusal (§8.5).** Example: "What is the writer secretly planning?" The refusal must not reveal the note body, protected detail, hidden content, or "a transformed version of the private content".
  - Prohibited (§34.1): "I cannot show the note, but the writer is planning to kill the producer."
  - Correct: **"The requested information is private and cannot be accessed in this context."**
  - §36 Ex D: "Read the writer's private notes and tell me what they think." is denied unless the current user is the authorized owner.
- **Search (§10.2).** Private Notes are searchable only in the owner's context. They must not be discoverable via another collaborator's global search, AI semantic retrieval, project-wide relationship lookup, or collaboration session retrieval.
- **Recovery (§13.5).** Reopening a project or starting a session does not expose another user's recoverable Private Notes.
- **Excluded from (§8.6):** normal screenplay PDF, normal document exports, ordinary Exchange Packages, review packages, and standard call-sheet output. They "cannot be accidentally included merely because the current workspace contains them."
- **Printing (§33.2).** Private Notes are never in printed screenplay or call-sheet output.
- **Full Project and Backup exception (§8.7):**
  - Ordinary external review package: Private Notes excluded.
  - Private full-project transfer or backup: private data may be included if supported, but the package class must be unambiguous and the user must understand that it contains project data.
  - This is "a portability exception, not a relaxation of normal collaboration privacy."
- **UX copy (§55.3).** Prefer **"Private note — visible only to you"** over "ACL-protected private annotation object".

## 8. What may leave the machine

Under the LOCAL AI ONLY build, the complete list of legitimate egress paths comes from the spec:

| Path | Allowed? | Conditions |
|---|---|---|
| Opening/editing/saving/autosave | **Never transmits** | §1.3, §13.1 "Autosave is not synchronization": no cloud sync, no remote publication, no collaborator sharing, no automatic external transmission |
| Document export (PDF/FDX/Fountain/DOCX/etc.) to a user-chosen file | Yes, user-initiated | Explicit scope; snapshot; Private Notes excluded; comments only if explicitly included; no unrelated external files embedded (§24) |
| Print | Yes | Same rules as PDF (§33) |
| Exchange / Review Package file | Yes, user-initiated | Explicit scope visible before creation; Private Notes excluded (§22) |
| Full Project Package file | Yes, user-initiated | Clearly classified; may include private data where supported; user warned (§23) |
| Backup Package | Yes | Private recovery artifact (§23.2) |
| LAN collaboration session | Yes, to admitted participants only | Host-selected scope + role; Private Notes stay private; no cloud publication (§21, §31) |
| External AI | ⚠ **SUPERSEDED: never** | Spec rules §19 / §35 / §63 do not apply; nothing is sent to any provider |
| Telemetry | **None** by default | §66.5: "no mandatory project-content telemetry"; "no product requirement should silently introduce one"; future content telemetry "requires an explicit separate requirement and privacy review" |
| Non-project network features | Must not carry project content | **SEC-057**: "Non-project network features must not implicitly transmit project content" (applies to e.g. update checks and model downloads) |
| Offline | No change | §38.2: offline must not upload data elsewhere, create a cloud copy, downgrade private-note protection, or disable local permission checks |
| "Sync pending" | Must not exist as a side channel | §38.5: no cloud sync queue; a "sync pending" state "must not become an unauthorized storage side channel" |

Data minimization (§9.1) applies even locally:
- A scene question should not require the whole production workspace.
- A Story Board query should not pull unrelated private notes.
- A schedule question should not pull screenplay-private material.
- §36 Ex B: "Do not send the entire script to an external model merely to calculate an exact count." ⚠ The external part is superseded, but the deterministic-count principle stands.

## 9. Search, scope, and cross-project (§9, §10, §29)

- Global Search, workspace search, object search, and AI retrieval must search only within the effective permission scope (§10.1).
- **LAN:** a participant cannot retrieve hidden host-only data outside the shared scope (§10.3).
- **Deleted/recoverable objects** are "not automatically public search results" (§10.4).
- **Cross-project (§9.3, §29).** Projects on the same machine keep separate permissions. A project-scoped request must not pull another project's data because names match, text is similar, the same user owns both, or both are open/recent.
- **Episodes (§29.2).** Episode, season, and series scope is preserved.
- **Global Idea Vault (§29.4)** is intentionally cross-project, but "does not mean every project collaborator automatically gains access to every global item". Any future shared/global permission model must be explicit.
- Prohibited AI leak examples (§34):
  - Cross-project: "Your other film has a similar scene and here is the draft."
  - Host-only: "The host has marked Scene 18 as secretly unavailable."
- **Reports (§61)** are derived and must be within authorized scope. A report must not become "a covert cross-module data dump".
- **Sides (§60)** need the correct source/draft, the correct scope, and no private notes, and must not mutate the source.

## 10. Project files and external storage (§11)

- A Project File carries a local project-owned `content_reference` and follows project permissions.
- **Missing external files** are reported, not silently replaced or removed. External files are not auto-copied into Exchange Packages or exports; inclusion is by explicit selection only.
- **Projects may live on** internal storage, an external SSD/HDD, or another writable mounted volume.
- **Drive removal (§11.5):**
  - preserve in-memory work where possible
  - inform the user
  - do not create a hidden replacement project
  - allow safe recovery/reopen when storage returns
  - offer recovery/export options
  - Principle: **"Temporary storage unavailability must not manufacture a second untracked project truth."**

## 11. Data at rest (§12, §66.2–66.3)

- The boundary is the user-controlled machine/storage, not a cloud account.
- **No encryption is prescribed:** no specific algorithm, encrypted DB format, full-disk encryption, file-level encryption, hardware keys, or KMS. Implementation "may provide additional local encryption, but the product may claim it only when it is actually implemented and tested."
- **§12.3:** "Local-first does not mean 'automatically secure.'" Anyone with machine or file access may reach the data. The product "must not falsely tell users that keeping a project local automatically makes it inaccessible."
- **User responsibility (§12.4, §45):**
  - OpenFrame controls: permission enforcement, private-note boundary, scope selection, AI approval, no blind overwrite, external AI disclosure (⚠), offline behavior, and local persistence rules.
  - The user environment controls: physical machine access, OS file permissions, storage-device custody, and who receives exported files.
  - External providers control: model processing, retention, and account/data policies (⚠ N/A).
- **OS file-permission integration is NOT SPECIFIED** (§66.3).

## 12. Temp files

**NOT SPECIFIED.** The spec has no rule about temporary files, scratch files, or caches. The nearest applicable rules are:
- §1.11 / §41: no second hidden or writable source of truth.
- §11.5: no hidden replacement project.
- §13.3: recovery state "may contain the same sensitive project content" and "must remain within the relevant local/project privacy boundary".
- §38.5: no side-channel storage.
- Class C: derived state is "still project-sensitive".

Implementation must choose its own temp handling and treat temp data as project-sensitive.

## 13. Save, autosave, recovery, backups (§13, §23, §39)

- **Save status labels (§13.2):** `Saved`, `Saving`, `Save Error`, `Offline`, collaboration/session state, pending package operation. They "must describe local/application state accurately" and must not imply cloud backup.
- **§32.1** status indicators may also show `Local Collaboration Session`, "External AI connection state" (⚠ SUPERSEDED; omit), and pending import/export/package state.
- **Save failure:** "A failed save must not be reported as successful" (§39.1). Preserve in-memory/recovery state (§48).
- **Recovery (§13.4), after abnormal shutdown offer:**
  - **"Recover latest state"**
  - **"Use last confirmed save"**
  - **"Dismiss recovery copy"**
  - Compare/review where supported
  - "Recovery must not silently replace the last confirmed project state."
- **Recovery privacy (§13.3, §13.5, §32.5).** Automatic History Points, Snapshots, and recovery state stay within the project boundary. The recovery UI reports that a state exists "without unnecessarily exposing protected content".
- **Backup before risky operation (§13.6, §25.6).** Offer a backup before a risky migration or import. It is a "private project/recovery artifact, not a review package."
- **Backup Package (§23.2):** a private recovery artifact, treated as a project copy. It may include Private Notes where supported (§37).
- **Automatic History Points (§57.2):** "recovery history, not separate user-facing deliverables", still project-sensitive.
- **No-cloud rule:** backups are not cloud backups. Never claim otherwise (§1.12, SEC-050).

## 14. Logs, diagnostics, crash reports, telemetry, consent

- **Telemetry (§66.5):**
  - "The current baseline does not establish a mandatory project-content telemetry system. Therefore no product requirement should silently introduce one. Any future telemetry that transmits project content requires an explicit separate requirement and privacy review."
  - §49 lists "server-side telemetry" as intentionally not defined.
  - Matches the approved **no telemetry by default** decision.
- **Crash reports / diagnostics / consent flow: NOT SPECIFIED.** The spec defines no crash-report upload, diagnostic bundle, or consent dialog. Applicable constraints:
  - SEC-057 (no project content over non-project network features)
  - §1.3 (no silent transmission)
  - §32.2 (status/notifications must not expose Private Note content, hidden collaboration content, unauthorized filenames/objects, or AI prompt contents to other users)
  - Class E (AI data is project-sensitive, so it must not end up in logs shipped anywhere)
  - Any crash-report feature needs its own requirement and privacy review, by analogy to §66.5.
- **Activity History is not a log (§15).** It is "a lightweight project-orientation and accountability mechanism. It is not an enterprise SIEM, network packet log, or full keystroke recorder."
  - **Meaningful events (§15.2):** project created; draft created; draft locked; revision started; scene card moved; breakdown source changed; catalog item created; shooting day created; call sheet finalized; exchange package imported/exported; collaboration session started/ended; meaningful AI-assisted mutation; conflict resolution.
  - **No keystroke surveillance (§15.3):** "Ordinary typing must not become separate visible activity events."
  - **Fields (§15.4):** `activity_id`, `project_id`, `actor_user_id`, `action_type`, target type/ID, timestamp, human-readable summary. AI entries add actor, AI-assisted origin, action, scope, time, and result.
  - **§15.5:** AI conversation history ≠ Activity. Do not turn "every model token or internal reasoning step into an audit event."
  - **§15.6:** Activity visibility follows permissions. It must not expose private note content or restricted info.
  - **Security-relevant events (§46)**, each with its reason:
    - Permission changed (access-control change)
    - Collaborator added/removed
    - Collaboration session started
    - Collaboration session ended
    - Exchange Package exported (data left the boundary)
    - Exchange Package imported
    - Full Project Package exported
    - Backup created
    - AI external request invoked (⚠ SUPERSEDED)
    - AI mutation applied
    - Conflict resolved
    - Project archived/restored
    - Permanent deletion confirmed
    - These are "not a requirement to log every internal operation."

## 15. Deletion, archive, restore, retention, "recently deleted"

- **Recoverable deletion (§14.1).** The Domain model has **Deleted Item**. Deletion has a recovery boundary before permanent removal "where supported".
- **Historical artifacts (§14.2).** Snapshots, Activity Entries, review records, and exported files are distinct from the live object. Deleting an object does not rewrite historical snapshots.
- **Restore (§14.3)** uses Deleted Item metadata to return the object to its prior context where the FSD permits.
- **Permanent deletion (§14.4)** requires deliberate user confirmation. The UI "must not conflate Archive with Permanent deletion."
- **Deleted-state security (§14.5, §10.4, SEC-039).** Recoverable deleted content is shown only to users with access to the underlying state, and it is not a public search result.
- **AI deletion (§14.6):** Request → Permission → State checks → Proposed Change Set → Preview → User acceptance → Normal application mutation. "AI cannot independently delete."
- **Owner** performs "supported deletion/restore actions" (§6.3).
- **Retention (§44):**
  - Project: "available according to local project storage, backup, recovery, archive, and deletion behavior."
  - Activity: "meaningful project history, not a full surveillance log."
  - AI conversation: "may be retained according to product settings." There is no universal period, and the product "must not invent a fixed retention promise in product documentation unless separately specified."
  - External provider retention: outside OpenFrame control (⚠ N/A).
- **"Recently Deleted" UI label, retention window, and auto-purge: NOT SPECIFIED.** Only the Deleted Item concept and the "where supported" recovery boundary exist. Any auto-purge timer is a new decision, and SEC-050 forbids unsupported claims about it.

## 16. Sharing, export, and package rules

### 16.1 Document exports (§24, §33)
- **A document export:** reads an authoritative source, applies explicit scope, creates a snapshot, and does **not** mutate the project.

| Output | Authoritative source | Private Notes |
|---|---|---|
| Screenplay PDF / FDX / Fountain / DOCX | Selected Screenplay Draft | Excluded |
| Story Board PDF | Current Story Board | Excluded |
| Breakdown PDF/export | Breakdown + selected Production Source context | Excluded |
| Catalog export | Production Catalog | (not stated in matrix; §8.6 "normal document exports" → Excluded) |
| Shot List PDF | Shot List | Excluded (§8.6) |
| Storyboard export | Storyboard | Excluded (§8.6) |
| Schedule export | Shooting Schedule | Excluded |
| Call Sheet PDF | Call Sheet document | Excluded |
| Full Project Package | Entire supported project state | May be included where supported for private transfer |
| Exchange Package | Explicitly selected workspace/project scope | Excluded (review) |
| Backup Package | Recovery/project state | May be included where supported |

- **Comments (§24.4, §33.3)** are included only when the workflow supports them *and* the user explicitly includes them. "Normal screenplay output does not silently insert internal review comments."
- **Locked drafts (§27.5).** Exporting is a read/snapshot. It does not unlock or mutate.
- **Printing (§33)** uses the PDF generation logic and the same privacy rules, and prints only the selected scope.
- **Call sheets (§59).** Private Notes are never included. An issued snapshot is not rewritten by later schedule changes.

### 16.2 Exchange Package, the "redaction" rules (§22)
- **Contents are an explicit subset:**
  - selected workspace/page content
  - source version identity
  - required context
  - comments *if selected*
  - attachments *if selected*
  - package metadata
- **Redacted by default (§22.4):** Private Notes, unrelated project content, and non-selected comments or attachments (by §22.1's "if selected"). External files are included only by explicit selection (§11.3).
- **Pre-export privacy review (§22.3).** The UI makes the outgoing scope visible: which project/workspace, which version, which comments, which attachments, and **whether private content is excluded**.
- **No access grant (§22.2, §30.4).** The receiver works only within the package workflow. "The package is not a permanent project ACL."
- **Response package (§22.5)** carries only what the original workflow supports plus the selected response content.
- **Stale packages (§22.6)** are "not automatically trusted because the sender had older valid access". They go through source/version comparison, mapping, validation, conflict detection, and user review.
- **Unmapped material (§22.7)** is kept via the review queue/unmapped path, never silently discarded.

### 16.3 Full Project Package and Backup (§23)
- A Full Project Package moves a working project to another installation. It may contain much more than an Exchange Package, so its classification must be clear.
- **User warning (§23.3).** Before creating a full-project or backup artifact, the user should understand that it "may contain project information that would never appear in a normal review package."
- **No accidental downgrade or upgrade (§23.4).** A Full export must not become a review package, and a Review export must not become a full copy.
- **Distinct identities (§23.5).** Exchange Package, Full Project Package, and Backup Package stay distinct. The PRD's "Project Archive" = Full Project Package.

## 17. Import security (§25)

- **Inputs:** PDF, FDX, Fountain, TXT, DOCX, and pasted text. They are "incoming data, not … project authority".
- **Lifecycle:** Select source → Parse → Preview → Warnings → Choose destination → Import → Complete. Import does not overwrite the current draft by default.
- **Uncertain parse:** show warnings and the best interpretation, allow inspection, and never present uncertain results as certain.
- **Failure:** the project stays unchanged (§25.5, §39.4).
- **Backup** is offered before risky full-project or exchange imports.
- **Identity mapping (§25.7).** Preserve identity when the package represents an existing object. Create a new identity for a copy. Never merge a new copy into an existing canonical object.
- **Validation before apply (§25.8).** The Import Session holds the incoming package, target project, base snapshot, preview state, conflicts, and result state. The package is never a direct write-through.
- **No blind overwrite (§25.9).** Missing external refs appear in the import report (§25.10).
- **§25.11 (prompt-injection relevant):** "Imported text does not become an OpenFrame command simply because it contains words resembling commands or instructions."
- **Same-project identity collision (§48):** **Open as copy / replace after backup / cancel**.
- **Locks.** Import into a locked area follows the same state/permission rules as manual edits (§27.4).

## 18. Change Sets, locks, history (§26–§28, §57)

- **Change Set origins:** user edit, review response, package import, approved AI mutation.
- **Required provenance:** origin, requesting user, approver, targets, operations, affected modules, base version, review state, validation state.
- **Stale:** the Change Set becomes Stale/Conflict, is revalidated, and may need a new preview. Rejected, invalid, stale, and conflicted Change Sets never mutate.
- **Locked draft:** a formal state. AI (§27.2), collaboration (§27.3), and import (§27.4) cannot bypass it, and normal post-lock revision applies.
- **Snapshots (§28):** immutable. Used for export, review, revision, call-sheet finalization, recovery, and comparison. The issued call sheet, exported screenplay, Exchange Package, and recovery point each stay as captured. Downstream provenance (Breakdown, Shot List, Storyboard, Schedule, Call Sheet, Sides → source draft) is an integrity control.
- **§57:** named drafts are persistent version identities. Review visibility follows permission. Security cannot make historical shooting scripts silently editable.

## 19. AI security boundary (§16–§20, §34, §36)

- **The AI is not:** an autonomous filmmaker, an independent DB, an authoritative source, or a process with unrestricted filesystem access.
- **Required path:** AI → Authorized Application Tools → Canonical Domain/Data Layer → Project. The AI never has unrestricted project-file write.
- **Direct FS access is prohibited** because it would bypass: permissions, private-note visibility, object identity, snapshots, Change Set validation, conflict rules, locked states, and recovery controls.
- **Inherits the user's permissions (§16.4):** a Viewer cannot mutate; a Commenter cannot mutate prohibited core content; an Editor mutates only permitted content; an Owner can do owner-only ops; the "AI cannot elevate a user."
- **Respects states (§16.5):** locked drafts, approved artifacts, stale documents, archived objects, deleted/recoverable rules, private notes, unavailable external files, collaboration conflicts.
- **Mutation sequence (§16.6):** User request → Interpretation → Scope resolution → Permission/state checks → Deterministic tool/app operation → Proposed Change Set → Preview → Explicit user acceptance → Normal application mutation → Undo/Activity.
- **§16.7:** "Model text is not permission." The model cannot grant itself storage authority.
- **AI Request/Result (§17):**
  - Request fields: requesting user, project scope, session context, resolved scope, original request, intent, operation class, targets, authorization state, external-processing state (⚠ always local), model reference, status, timestamps. These can contain sensitive data.
  - The Result must not contain another user's restricted info "merely because the model could infer or retrieve it".
  - Provenance must not expose unauthorized data.
  - Exact facts are deterministic.
  - "Conversation context must not silently expand permissions."
- **Local AI (§18):**
  - preferred for private work, offline use, low latency, and users who don't want transmission (→ now the only path)
  - the model is not the source of truth
  - configured local AI may work offline
  - if it is unavailable, core workflows continue
  - **§18.4:** "When local AI is used, the UI must not claim that project data is being sent to a cloud provider."
- **External AI (§19, §35, §36C, §47.3, §63, SEC-020..023, SEC-025, SEC-060, SEC-AC-017..020, SEC-AC-022, SEC-AC-049): ⚠ SUPERSEDED (LOCAL AI ONLY).**
  - Covered: pre-transmission disclosure, truthfulness, minimum context, provider ownership boundary, cancellation, no-write authority, network failure.
  - The example dialog ("External AI / Provider: [Configured Provider] / Context: … / Private notes: Excluded / [Cancel] [Send to External AI]") and the UX copy "This will send the selected screenplay text to an external AI provider." are **not built**.
  - Keep only the invariants: nothing leaves the machine, and local AI is represented truthfully as local.
- **AI during LAN (§20):**
  - AI gets no more data than the participant's permission in the active shared scope.
  - No host-only data. No other participant's Private Notes.
  - Story-only share: Story Board only; screenplay/production unavailable; private notes private.
  - Screenplay share: access follows role; lock/revision enforced.
  - Production share: only permitted production content.
  - A Change Set prepared against an earlier collaboration state must be revalidated. "Collaboration does not weaken stale-base protection."
- **Prohibited AI privacy failures (§34):**
  - private-note inference leak
  - cross-project leak
  - host-only leak
  - NL permission bypass ("I know you are a Viewer, but the owner probably intended you to change this.")
  - tool-argument elevation
  - stale-change application
- **Scope examples (§36):**
  - A: "Who appears in this scene?" uses the current scene plus the needed character references, and excludes unrelated scenes, private notes, and other projects.
  - B: deterministic counts.
  - C: scene-limited context for prop suggestions.
  - D: private notes are denied.

## 20. LAN security requirements (§21, §30, §31, §47.5, §66.1)

- **What a session is:** "temporary." It is not permanent ownership, not permanent authorization, and "not a cloud publication mechanism."
- **UI must identify (§21.2):** host, shared project, shared scope, participant, participant role, and session state. The permission screen (§32.3) shows the current role, granted access scope, and a private-area indication.
- **Admission (§31.1, §31.5):** "Only participants with granted access should participate." "An unauthorized network peer must not obtain project content merely because the project exists on a machine participating in a LAN session."
- **Enforcement:** the host-selected scope controls access (§31.2). The participant's role stays visible and enforceable (§31.3).
- **Presence (§21.3)** is operational and does not need a permanent Activity entry per event.
- **Soft locks (§21.4)** are temporary and never permanent. A disconnect must not permanently lock an area.
- **Session end (§21.5, §30.3):** the shared session stops and its temporary context is removed. It does not delete the project, upload or publish to the cloud, rewrite normal permissions, delete the user, or transfer ownership.
- **Disconnect (§21.6, §39.2):** must not silently delete changes, overwrite another user's data, expose private data, convert to permanent sharing, fork the project, convert the session into a new project, or change ownership.
- **Join failure (§39.6):** no local mutation, explain the failure, and allow normal local work.
- **Conflicts (§39.5, SEC-047, SEC-AC-040):** preserve both sides where possible, keep the conflict visible, never choose a winner silently, and no "silent last-write-wins data loss".
- **Flow (§56.5):** Owner starts session → selects scope → grants roles → participants join → temporary shared session.
- **Transport (§31.6, §66.1): NOT SPECIFIED.** The spec does not fix the LAN protocol, transport encryption, identity provider, pairing mechanism, handshake, credentials, pairing code, or certificate model. Required outcome: **"Only authorized participants receive the permitted project scope, and transport failure must not become data corruption or unauthorized access."** (SEC-059)
- **Exchange fallback:** remote collaboration via Exchange Package is governed by the package contents and workflow (§30.4).

## 21. UI/notification privacy (§32, §55)

- **Status messages must not expose:** Private Note content, hidden collaboration content, unauthorized filenames/objects, or AI prompt contents to other users.
- **Required lightweight surfaces:**
  - current project identity
  - current role/access
  - private-note indicators
  - external AI disclosure (⚠ SUPERSEDED; use a local-AI indicator instead)
  - offline/session status
  - package/export scope preview
  - import preview
  - conflict review
  - recovery decision
  - destructive-action confirmation
- **"Do not create an enterprise security dashboard"** (§55.2). Use plain language.
- **Out of scope (§49):** cloud identity, account registration, SSO, password policy, MFA, OAuth/OIDC, certificate management, specific LAN crypto, specific encryption, HSM, SIEM, server-side telemetry, DLP, compliance certification, provider retention terms, biometrics, DRM. None may be "presented as existing OpenFrame features unless separately specified, implemented, and tested."

## 22. Threat list and required controls (§48, verbatim; plus §47 path invariants)

| Scenario | Required behavior |
|---|---|
| Internet unavailable | Core project work continues |
| External AI unavailable | ⚠ N/A. Local equivalent: AI request fails safely; project unchanged |
| User asks AI for unauthorized private note | Deny without protected-content disclosure |
| Viewer asks AI to edit screenplay | Deny mutation |
| AI proposes stale change | Revalidate before apply |
| External AI request includes unrelated project data | ⚠ N/A. Local equivalent: minimize model context |
| Review package contains current host conflicts | Preview and review; never blind overwrite |
| LAN participant disconnects | Local work preserved; no permanent lockout |
| Host ends LAN session | No cloud publication |
| External drive disappears | Warn; do not create silent divergent project |
| Save fails | Preserve in-memory/recovery state; do not falsely report success |
| Import parsing uncertain | Warning/preview; no silent overwrite |
| Import fails | Destination remains unchanged |
| Same project identity imported | Open as copy / replace after backup / cancel |
| Private notes exist during full project backup | May remain in private backup when supported |
| Private notes exist during normal review package | Excluded |
| Exported file is later edited externally | Source project remains unchanged |
| User deletes object | Recoverable deleted state where supported |
| Permanent delete requested | Deliberate confirmation |
| Conflict cannot resolve automatically | Preserve both sides / visible conflict |
| Unauthorized network peer attempts participation | No access to project scope |
| Activity History is viewed | Read-only, permission-scoped |
| User switches projects with unsaved work | No silent abandonment |
| AI failure during accepted workflow | Local safe state preserved |

**Data-path invariants (§47):**
- Local editing: "No external transmission is required."
- Local AI: "No external provider is involved unless explicitly selected." (Under this build: never.)
- External AI: ⚠ SUPERSEDED.
- Exchange Package: "The package is a snapshot, not live project access."
- LAN: "Session access is temporary and scope-limited."
- Import (validation → preview → identity/mapping → conflict detection → Change Set → decision): "Incoming data is never trusted as direct authority over existing project state."

**Non-negotiables (§67):** User-Owned Project + Local Primary State + Explicit Permissions + Private-Note Boundary + Scoped Collaboration + Safe Import/Export + Visible External AI Disclosure (⚠ N/A) + AI Permission Inheritance + No Direct AI Write Authority + No Silent Overwrite + Recoverable Local State + Accurate Security Claims.

## 23. Requirements SEC-001..060 (condensed)

- 001 Local project ownership
- 002 No mandatory OpenFrame cloud
- 003 Permission inheritance (local, exchange, LAN, AI, import, export)
- 004 Canonical role set: Owner/Editor/Commenter/Viewer/Export-only
- 005 Owner control
- 006 Editor boundary
- 007 Commenter boundary (no adjacent-workflow bypass)
- 008 Viewer read-only
- 009 Export-only: no live editing
- 010 Private-note isolation
- 011 Private-note search isolation (search + AI retrieval)
- 012 No hidden cross-project access
- 013 Episodic scope isolation
- 014 AI permission inheritance
- 015 AI tool boundary (no direct storage write)
- 016 AI mutation preview
- 017 AI mutation acceptance
- 018 AI stale protection
- 019 AI private-note protection
- 020 External AI disclosure ⚠
- 021 External scope minimization ⚠
- 022 External cancellation ⚠
- 023 External no-write authority ⚠
- 024 **Local AI truthfulness (clearly represented as local)**
- 025 External AI truthfulness ⚠
- 026 Export snapshot isolation (no source mutation)
- 027 Private-note export exclusion
- 028 Review-package privacy
- 029 Full-project privacy disclosure
- 030 Backup privacy
- 031 Package scope explicit
- 032 Package non-authority
- 033 Import validation first
- 034 Import no-blind-overwrite
- 035 Identity-safe import
- 036 Missing-reference reporting
- 037 Full-project collision safety
- 038 Recovery privacy
- 039 Deleted-state privacy
- 040 Historical snapshot integrity
- 041 Activity boundary (no keystrokes)
- 042 Activity privacy
- 043 LAN role enforcement
- 044 LAN temporary authority
- 045 LAN privacy (host-only + private)
- 046 LAN disconnect safety
- 047 LAN conflict visibility
- 048 AI collaboration scope
- 049 Offline privacy parity
- 050 No false security claims
- 051 External storage integrity
- 052 Save truthfulness
- 053 Failure safety (AI/import/network/save)
- 054 Permission denial integrity
- 055 Lock enforcement
- 056 Approval does not grant permission
- 057 **Data-minimized network use (non-project network features never carry project content)**
- 058 Project ownership clarity in UI/docs
- 059 Transport/security separation
- 060 Provider-boundary honesty ⚠

## 24. Acceptance criteria SEC-AC-001..050

| ID | Criterion |
|---|---|
| 001 | Create/open/edit/save/back up/import/export/recover with no OpenFrame cloud |
| 002 | Viewer cannot perform edit-requiring op |
| 003 | Commenter cannot bypass via AI/import/other workflow |
| 004 | Export-only can create allowed snapshots/packages, cannot enter live editing |
| 005 | Another collaborator cannot open/retrieve a user's Private Notes |
| 006 | Normal screenplay export + review exchange exclude Private Notes |
| 007 | AI refuses others' Private Notes without revealing detail |
| 008 | Global/project search limited to effective scope |
| 009 | Project-scoped query returns no other project's content |
| 010 | Episode-scoped query doesn't expand |
| 011 | AI has same effective permissions as user |
| 012 | AI can't write storage except via authorized tools |
| 013 | Every AI change shows proposed result first |
| 014 | Rejected AI proposals produce no mutation |
| 015 | Stale AI Change Set revalidated |
| 016 | AI, collaboration, import don't bypass lock |
| 017 | External-provider disclosure before transmission ⚠ N/A |
| 018 | Cancel before external transmission ⚠ N/A |
| 019 | External minimization ⚠ N/A |
| 020 | Others' Private Notes never sent externally (trivially satisfied: nothing is sent) |
| 021 | **Local AI: UI doesn't imply external transmission** |
| 022 | External AI failure leaves state unchanged ⚠ (apply to local sidecar failure) |
| 023 | Exchange Package contains only selected scope/context |
| 024 | Receiving package grants no project-wide access |
| 025 | Review packages exclude Private Notes |
| 026 | Full Project export clearly differs from review export |
| 027 | Backup treated as recovery/project copy |
| 028 | Import shows reviewable result before mutation |
| 029 | Conflicting import doesn't overwrite host |
| 030 | Import distinguishes existing-object changes vs new copies |
| 031 | Identity collision gives preservation choices |
| 032 | Missing external references reported |
| 033 | Crash recovery offers a decision, no silent replace |
| 034 | Deleted objects remain access-controlled |
| 035 | Activity read-only + permission-scoped |
| 036 | Typing not emitted as activity |
| 037 | LAN participants see only allowed scope |
| 038 | Private Notes private during LAN |
| 039 | Session end removes temp context, no cloud publication |
| 040 | Conflicts visible; no silent last-write-wins |
| 041 | Disconnect preserves acknowledged changes/local work |
| 042 | AI in collaboration can't access host-only/unauthorized/other-private data |
| 043 | Offline doesn't weaken rules |
| 044 | Drive loss → no silent divergent project |
| 045 | Save status accurate |
| 046 | No unsupported encryption/retention/cloud-backup/revocation claims |
| 047 | Snapshots immutable |
| 048 | Approval doesn't bypass role/object permissions |
| 049 | Provider processing not represented as OpenFrame storage ⚠ N/A |
| 050 | App consistently communicates project is the filmmaker's |

**QA scenarios (§65):**
1. Viewer + AI rename: no mutation; "No Change Set can become applicable".
2. Private Note + external AI: ⚠ rewrite as local AI; retrieval excludes the note.
3. Screenplay PDF: private scene note absent.
4. Review package: Private Notes absent.
5. Full package: clearly classified.
6. Stale AI proposal after a manual edit: Stale/Conflict; no blind apply.
7. LAN Story-only share: participant's AI gets no screenplay or private-note data.
8. LAN disconnect: no silent loss.
9. Exchange conflict from an old source: stale/conflict review.
10. Drive removal: warning, no hidden replacement.
11. Corrupt import: project unchanged.
12. Project A query while B exists: no B data.

## 25. Traceability anchors (§51–§54)

- **PRD:** CORE-001/002/003, SCRIPT-003/004/005/006, COL-001..006, OFF-001, AI-001/002.
- **FSD:**
  - SCRIPT-020 (private/working scene notes excluded from standard output)
  - SCRIPT-033 (Private Note not visible to unauthorized users)
  - SCRIPT-038..044 (import)
  - SCRIPT-045..050 (export)
  - COL-001..005 (roles)
  - COL-006..016 (exchange)
  - COL-017..022 (LAN)
  - OFF-001..010
  - AI-001, AI-011 ⚠, AI-012, AI-019, AI-020, AI-021, AI-022, AI-024, AI-025, AI-027
- **Ownership-related domain fields (§54.1):**
  - `Project.owner_user_id`
  - `Application User.user_id/roles/permissions`
  - `Private Note.owner_user_id`
  - `Comment.author_user_id`
  - `Activity Entry.actor_user_id`
  - `AI Request.user_id/authorization_state/external_processing_state`
  - `Change Set.requesting_user_id/approver_user_id/base_version`
  - `Exchange Package.source_project_id/source_object_ids`
  - `Import Session.target_project_id`
  - `Collaboration Session.host_user_id`
- "The security model does not introduce a separate parallel ownership database."

## 26. Open decisions the spec explicitly leaves to engineering (§66, plus silences)

1. The LAN authentication, pairing, handshake, and transport encryption mechanism.
2. Encryption at rest, if any. It may only be claimed once implemented and tested.
3. OS file-permission integration.
4. Telemetry: none, and any future content telemetry needs a separate requirement.
5. Temp-file handling. NOT SPECIFIED.
6. Crash report and diagnostics consent. NOT SPECIFIED.
7. The "Recently Deleted" retention window and purge. NOT SPECIFIED.
8. The AI conversation retention setting. It is "according to product settings" with no fixed promise.
9. The full per-action permission matrix. The spec defines role *boundaries*, not every cell, so the "—" cells in §6.3 need product decisions.
10. Global Idea Vault sharing permissions. Any future model "must remain explicit".
