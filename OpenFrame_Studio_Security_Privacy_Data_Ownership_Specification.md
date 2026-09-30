# OpenFrame Studio — Security / Privacy / Data Ownership Specification

**Document type:** Cross-cutting security, privacy, data-ownership, access-control, disclosure, project-file, collaboration, AI-data, import/export, recovery, and lifecycle specification

**Product:** OpenFrame Studio

**Platform:** Windows + macOS desktop application

**Operating model:** Local-first, offline-capable, user-owned project files; optional local AI; optional external AI; optional local-network collaboration; no mandatory OpenFrame cloud

**Current source baseline reviewed:**
- `OpenFrame_Studio_Mega_PRD_Aligned_Updated(2).md`
- `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated(2).md`
- `OpenFrame_Studio_UX_UI_Specification_Updated(2).md`
- `OpenFrame_Studio_Domain_Data_Specification_Updated(2).md`
- `OpenFrame_Studio_AI_Specification_Updated(2).md`
- `OpenFrame_Studio_Import_Export_Specification(1).md`
- `OpenFrame_Studio_Offline_Collaboration_Specification(1).md`

**Purpose:** Define the product-level contract for protecting user-owned OpenFrame projects, preserving privacy boundaries, enforcing access permissions, controlling external transmission, preventing unauthorized mutation or disclosure, and keeping data ownership explicit across local work, offline use, AI, import/export, backups, Exchange Packages, and local-network collaboration.

**Status:** Cross-cutting Security / Privacy / Data Ownership baseline.

> This specification specializes security, privacy, and ownership behavior already implied by the product documents. It does not replace the PRD's product scope, the FSD's functional behavior, the UX/UI specification's presentation contract, the Domain/Data specification's logical model, the AI specification's AI boundary, or the specialized Import/Export and Offline/Collaboration specifications.

---

# 0. Specification Contract

## 0.1 Why this specification exists

OpenFrame Studio is deliberately designed around local ownership rather than mandatory cloud dependency. That architectural/product choice creates a different security and privacy model from a cloud-first collaborative application.

The central principle is:

> **The filmmaker owns the project; OpenFrame is the workspace.**

Security therefore cannot be reduced to “protect the cloud account,” because ordinary OpenFrame use does not require such an account. The product must instead protect:

- local project access;
- user/project permissions;
- private notes and private context;
- project data exposed through AI;
- data included in document exports;
- data included in Exchange Packages;
- data included in full-project and backup packages;
- data visible during a LAN collaboration session;
- recovery and historical state;
- imported content;
- application activity/history;
- external-provider disclosure and transmission.

## 0.2 Authority and alignment

The cross-document relationship is:

```text
PRD
  ↓
FSD
  ↓
┌─────────────────────────────────────────────────────────────┐
│ UX/UI │ Domain/Data │ AI │ Import/Export │ Offline/Collab   │
│       │             │    │                │                │
└──────────────────────────────┬──────────────────────────────┘
                               ↓
                 Security / Privacy / Ownership
                     cross-cutting specialization
```

This diagram is an alignment model, not a claim that all specialist documents replace the FSD.

The governing rules are:

1. The PRD owns product intent, scope, priorities, and boundaries.
2. The FSD owns observable functional behavior.
3. UX/UI owns presentation and user-facing interaction.
4. Domain/Data owns logical identities, ownership, relationships, versioning, and source-of-truth relationships.
5. AI owns AI-specific data access, command, permission, disclosure, and approval behavior.
6. Import/Export owns portability, snapshot, package, mapping, and exchange safety.
7. Offline/Collaboration owns offline, local storage, LAN session, exchange collaboration, permission, presence, and conflict behavior.
8. This specification owns the security/privacy/data-ownership interpretation of those behaviors.
9. No specialist document may create an independent source of truth for project content.
10. No security control may silently change approved product scope.

## 0.3 Security, privacy, and ownership are different concepts

### Security

Security answers:

> **Who is allowed to access or mutate this information or state?**

### Privacy

Privacy answers:

> **What information is allowed to be disclosed, transmitted, exported, or shown in this context?**

### Data ownership

Ownership answers:

> **Who controls the project as a user-owned working asset, and what does sharing actually grant?**

These concepts overlap, but they are not interchangeable.

A user may own a project while giving another person Viewer access.

A user may have Editor access while still being prohibited from seeing another user's Private Notes.

A package recipient may receive a screenplay review package without receiving project-wide access.

An external AI provider may process selected project context without becoming the owner of the OpenFrame project.

---

# 1. Core Security / Privacy / Ownership Principles

These are normative.

## 1.1 Local ownership

The saved local project is the primary project state.

OpenFrame must not behave as though the platform owns the user's project.

## 1.2 No mandatory cloud account

Ordinary project creation, opening, editing, saving, importing, exporting, backup, recovery, and core filmmaking work must not depend on an OpenFrame cloud account.

## 1.3 No silent transmission

Project information must not be transmitted outside the local application merely because a user opened a project, edited a screenplay, or used a normal offline-capable workflow.

## 1.4 Explicit external AI disclosure

When external AI is used, the user must be informed before project information is sent outside OpenFrame.

## 1.5 Scope-limited disclosure

Only the context required for the requested external AI operation should be transmitted.

## 1.6 Private-note protection

Private Notes are a stronger privacy boundary than ordinary project content.

Another collaborator's Private Notes must never be surfaced through AI or collaboration merely because those notes exist inside the same project.

## 1.7 Permission inheritance

The effective permission of the current user applies consistently to:

- normal local project use;
- Exchange Packages;
- LAN collaboration;
- AI;
- import;
- export.

## 1.8 User-controlled mutation

A request, including a natural-language AI command, does not automatically authorize a project mutation.

Project-changing AI actions still require the normal OpenFrame preview and explicit acceptance workflow.

## 1.9 No direct AI storage authority

AI does not receive unrestricted filesystem access or direct project-write authority.

AI routes through authorized application tools and the canonical domain/data layer.

## 1.10 No silent overwrite

Import, exchange reconciliation, collaboration conflict resolution, recovery, and other high-risk operations must not silently destroy an existing user state.

## 1.11 Recoverability

Security controls must not create a second hidden source of truth.

When something fails, the product should preserve recoverable local state and make the resulting state visible to the user.

## 1.12 No false security claims

The product must not claim encryption, cloud backup, secure synchronization, provider retention behavior, or other security guarantees that are not actually implemented and represented in the product contract.

---

# 2. Data Ownership Model

## 2.1 What “user-owned project” means

A project consists of the user's filmmaking work represented by the OpenFrame domain model, including as applicable:

- Idea Vault content;
- Story Board content;
- screenplay drafts and scenes;
- comments;
- Private Notes;
- production breakdowns;
- catalog data;
- locations;
- cast/crew records;
- moodboards;
- storyboard panels;
- shots;
- shooting schedules;
- call sheets;
- project notes;
- tasks;
- lightweight budget information;
- project files;
- project history/recovery state;
- supported project metadata.

The user's local project file is the primary saved working asset.

## 2.2 OpenFrame is not the owner

OpenFrame provides the application workspace, data model, editing tools, export tools, collaboration tools, and optional AI services.

It must not present ordinary project data as if it were owned by OpenFrame.

This directly reflects the PRD product design decision that the project is the filmmaker's work and OpenFrame is the workspace.

## 2.3 Ownership does not mean unrestricted access by every collaborator

The owner of a project can share permitted access.

Sharing does not erase:

- role restrictions;
- project scope;
- object/state restrictions;
- private-note boundaries;
- screenplay lock behavior;
- package boundaries;
- AI permission boundaries.

## 2.4 Ownership does not automatically transfer through export

A document export creates a snapshot.

An Exchange Package is a portable collaboration/review snapshot.

A Full Project Package is a portable working-project transfer concept.

A Backup Package is a recovery-oriented copy.

None of these should be described as “OpenFrame taking ownership” of the source project.

## 2.5 Opening a full project package as a copy

When a Full Project Package is opened as a copy, the destination must distinguish the copy from the original project identity.

The Domain/Data rule that duplicated/copied content receives new identity applies to the copied project context.

The source identity may remain as provenance where needed, but the copied project must not accidentally become a second live write path to the original project.

## 2.6 Replacing an existing project

A “Replace Existing After Backup” operation is a deliberate local replacement operation.

It is not normal synchronization.

It does not imply that the source package has gained permanent control over the destination project.

The replacement must remain subject to the destination user's effective permissions and the existing import safety rules.

---

# 3. Security Scope

Security applies across the entire project lifecycle:

```text
Create
  ↓
Edit
  ↓
Autosave / Manual Save
  ↓
History / Recovery
  ↓
Export
  ↓
Share / Exchange
  ↓
Import / Reconcile
  ↓
Collaboration
  ↓
Archive / Delete / Restore
```

It also applies across processing boundaries:

```text
Local Project
   ├── Local Application
   ├── Local AI
   ├── External AI
   ├── Local Network Collaboration
   ├── Document Export
   ├── Exchange Package
   ├── Full Project Package
   └── Backup Package
```

Each boundary must preserve the user's effective permissions and relevant privacy rules.

---

# 4. Data Classification Model

The existing product specifications do not define a large enterprise information-classification taxonomy. OpenFrame should therefore use a deliberately small product-level model.

## 4.1 Class A — Project content

Ordinary project information such as:

- story structure;
- screenplay content;
- breakdown data;
- production data;
- schedule data;
- call-sheet data;
- project notes and tasks;
- project files;
- approved or historical project documents.

Access follows project permissions and workflow scope.

## 4.2 Class B — Private project content

Private Notes are explicitly private.

They are visible only to the owner/authorized private context.

This class must not be exposed merely because another user is allowed to access the surrounding project.

## 4.3 Class C — Derived or historical project state

Examples:

- Snapshots;
- Automatic History Points;
- deleted/recoverable state;
- issued call-sheet snapshots;
- exported snapshots;
- exchange snapshots;
- recovery state.

These are still project-sensitive.

They are not automatically less private just because they are historical.

## 4.4 Class D — Collaboration context

Examples:

- Collaboration Session;
- host identity;
- participant identity;
- granted role;
- shared scope;
- presence;
- temporary locks;
- conflict state.

This information exists to operate a collaboration session and should not be treated as project-wide permanent content.

## 4.5 Class E — AI interaction data

Examples:

- AI Request;
- request text;
- selected project scope;
- resolved target objects;
- AI Result;
- suggested Change Set;
- provenance;
- external-processing state;
- provider/model reference where applicable.

AI interaction data may itself contain project-sensitive content and must therefore follow the relevant user/project privacy boundary.

## 4.6 Class F — Local profile/context data

The Domain/Data model includes a local Application User with:

- stable user identity;
- display name;
- roles;
- local profile;
- effective permissions.

This is an application identity/context object, not an enterprise HR identity system.

---

# 5. Application User and Identity Boundary

## 5.1 Canonical Application User

The canonical user concept is `Application User`.

The current Domain/Data specification defines:

- `user_id` — stable local identity;
- `display_name` — visible user name;
- `roles` — project roles where applicable;
- `local_profile` — user preferences where applicable;
- `permissions` — effective access derived from role/context.

## 5.2 Stable identity

A user's stable local identity should not change simply because the display name changes.

Similarly, renaming a project object does not change that object's identity.

Identity stability is important for:

- activity attribution;
- comment authorship;
- private-note ownership;
- collaboration participation;
- change-set responsibility;
- review provenance.

## 5.3 Authentication is not fully specified by the current product baseline

The current documents define authorization behavior and local user identity, but they do not define a complete enterprise authentication system.

Therefore:

- ordinary local use must not require an OpenFrame cloud login;
- collaboration must admit only users who have been granted access to the session;
- the precise technical authentication mechanism for LAN admission is an implementation/security design decision;
- this specification does not invent passwords, accounts, identity providers, or token formats that are not present in the product baseline.

The security requirement is the observable result:

> **An unauthorized participant must not gain the permissions or project scope of an authorized participant.**

---

# 6. Canonical Authorization Roles

## 6.1 Security roles

The canonical collaboration/access-control roles are exactly:

| Role | Access meaning |
|---|---|
| Owner | Full project control |
| Editor | Edit authorized project content |
| Commenter | View permitted content and comment/review where allowed |
| Viewer | Read-only |
| Export-only | Create permitted snapshots/packages without editing project data |

These roles are the security model.

## 6.2 Professional responsibilities are not security roles

The PRD may describe contextual filmmaking responsibilities such as Writer, Director, Producer, Assistant Director, Reviewer, or Contributor.

Those labels must not silently replace the canonical access-control role model.

A user may be a Writer and Owner.

Another user may be a Director and Editor.

Security authorization is still evaluated using the effective access role/context defined by the product's permission model.

## 6.3 Owner

The Owner can:

- manage project settings;
- manage permissions;
- manage collaboration session ownership;
- edit permitted content;
- perform owner-only destructive operations;
- end the collaboration session;
- perform supported deletion/restore actions.

Owner authority remains subject to object/state restrictions that the application defines for safety.

## 6.4 Editor

An Editor may edit authorized project content.

An Editor does not automatically gain:

- ownership;
- permission-management authority;
- unrestricted destructive control;
- private-note access belonging to another user;
- the ability to bypass locked or approved states.

## 6.5 Commenter

A Commenter may:

- view permitted content;
- add comments;
- reply/resolve comments where allowed.

A Commenter cannot use comments, AI, import, or collaboration mechanisms as a route to edit core screenplay/story/production content when the permission model prohibits it.

## 6.6 Viewer

Viewer is read-only.

A Viewer cannot turn a read operation into a mutation by:

- AI command;
- import;
- exchange response;
- collaboration;
- script lock interaction;
- export workflow.

## 6.7 Export-only

Export-only allows permitted snapshots/packages without editing project data.

Export-only users cannot enter a live editing session.

Export-only access is not project ownership.

---

# 7. Permission Evaluation Model

## 7.1 Effective permission

The effective permission is derived from the user's role/context and the current collaboration/package scope.

Authorization must consider at least:

```text
User
+
Project
+
Role
+
Shared Scope / Package Scope
+
Target Object
+
Object State
+
Operation
```

## 7.2 Permission before mutation

Before a mutation occurs, the system must evaluate:

1. requesting user;
2. current project;
3. requested scope;
4. target object(s);
5. current object state;
6. effective role/permission;
7. whether the operation is allowed;
8. whether an additional confirmation/approval gate is required.

## 7.3 Permission is not implied by visibility alone

Being able to see an object does not necessarily mean the user may:

- edit it;
- delete it;
- export all related data;
- share it;
- change permissions;
- unlock a draft;
- apply an AI Change Set.

## 7.4 Permission denial

A denied operation must:

- not mutate project data;
- clearly explain the blocked operation at the appropriate UX level;
- not silently downgrade into a different write operation;
- not reveal protected private information merely to explain the denial.

Where a protected object's existence itself is private, the product should not disclose details beyond what the permission model permits.

---

# 8. Private Notes

## 8.1 Private Notes are a first-class privacy boundary

The Domain/Data model defines Private Note with:

- `private_note_id`;
- `owner_user_id`;
- optional target;
- body;
- timestamps.

The essential rule is:

> **Private Notes belong to their owner/authorized private context.**

## 8.2 Private Notes in ordinary collaboration

Private Notes remain private during:

- Solo Local operation;
- Exchange Package preparation;
- Exchange Package import;
- LAN collaboration;
- AI interaction;
- export;
- review;
- offline use.

## 8.3 Private Notes are not a hidden shared database

A user selecting an entire project for collaboration does not automatically convert Private Notes into shared content.

The normal application must be used to deliberately transform information into ordinary shared project content.

## 8.4 Private Notes and AI

A user may ask AI to use their own Private Notes when:

- the user is the owner/authorized private context;
- the request is permitted;
- the relevant context is intentionally included.

AI must never expose another collaborator's Private Notes.

## 8.5 Protected-content response

When the assistant is asked:

> “What is the writer secretly planning?”

and the only relevant source is another user's Private Notes, the assistant must refuse that data access without disclosing the protected content.

The response must not reveal:

- the private note body;
- protected detail;
- hidden private-note content;
- a transformed version of the private content.

## 8.6 Private Notes and exports

Private Notes are excluded from:

- normal screenplay PDF;
- normal document exports;
- ordinary Exchange Packages;
- review packages;
- standard call-sheet output.

They cannot be accidentally included merely because the current workspace contains them.

## 8.7 Full Project Package / Backup exception

The Import/Export specification distinguishes ordinary external review packages from private project transfer/backup concepts.

A Full Project Package or Backup Package may contain supported private project data when the user intentionally treats the package as a private project transfer or recovery artifact.

Therefore the rule is:

```text
Ordinary external review package
    → Private Notes excluded

Private full-project transfer / backup
    → Private data may be included if supported
    → Package class must be unambiguous
    → User must understand that the exported package contains project data
```

This is a portability exception, not a relaxation of normal collaboration privacy.

---

# 9. Data Minimization and Scope

## 9.1 Minimum necessary context

OpenFrame should expose only the context required for the current operation.

Examples:

- a scene question should not require the entire production workspace;
- a Story Board query should not require unrelated private notes;
- a schedule question should not automatically transmit screenplay-private material;
- an external AI request should not transmit unrelated project modules.

## 9.2 Current context versus project-wide context

The AI specification permits project-wide questions but requires scope to be identified when it materially affects the answer.

Security and privacy must follow the resolved scope.

The user's ability to ask:

> “How many scenes are there?”

does not grant AI unrestricted access to data outside the project/episode scope necessary to calculate that result.

## 9.3 Cross-project boundaries

AI and ordinary application searches must not silently cross project boundaries.

The AI specification explicitly requires cross-project/episodic scope control.

A user asking about one project must not receive another project's content merely because both projects exist on the same machine.

---

# 10. Search and Retrieval Privacy

## 10.1 Search follows permission

Global Search, workspace search, object search, and AI retrieval must search only data available within the user's effective permission scope.

## 10.2 Private Notes

Private Notes must be searchable only within the owner's/authorized private context.

They must not become discoverable through:

- global search for another collaborator;
- AI semantic retrieval;
- project-wide relationship lookup;
- collaboration session retrieval.

## 10.3 Hidden host data

In LAN collaboration, a participant must not retrieve hidden host-only data outside the shared scope.

AI during collaboration is explicitly constrained by the participant's current permission and shared scope.

## 10.4 Deletion and recoverable state

Recoverable deleted objects are not automatically public search results.

Access to deleted/recoverable data must follow the normal lifecycle and authorization rules.

---

# 11. Project Files and External References

## 11.1 Project File ownership

The Domain/Data model defines Project File as a project document/attachment with a local project-owned `content_reference`.

It is part of project data and must follow project permissions.

## 11.2 External files

The product distinguishes project-contained files from external file references.

Missing external files must be reported rather than silently replaced or removed.

## 11.3 Security consequence of external references

An external file is not automatically copied into an Exchange Package or ordinary document export.

Its inclusion must follow the relevant export/package workflow and user selection.

## 11.4 External-drive projects

A project may live on:

- internal storage;
- external SSD/HDD;
- another supported writable mounted volume.

The project remains the same logical project while that storage is available.

## 11.5 Drive removal

If the project volume disappears:

- preserve in-memory work where possible;
- inform the user;
- do not create a hidden replacement project;
- allow safe recovery/reopen when storage returns;
- offer supported recovery/export options where appropriate.

The security principle is:

> **Temporary storage unavailability must not manufacture a second untracked project truth.**

---

# 12. Local Storage and Data at Rest

## 12.1 Primary local boundary

The current product baseline defines local file ownership and local persistence as the core model.

Therefore the principal project-data boundary is:

```text
User-controlled machine / storage
        ↓
OpenFrame project
```

rather than:

```text
OpenFrame cloud account
        ↓
Project
```

## 12.2 No unsupported encryption claim

The current product specifications do not prescribe:

- a specific encryption algorithm;
- an encrypted database format;
- full-disk encryption;
- file-level encryption;
- hardware-backed keys;
- a key-management service.

This security specification therefore does not invent one.

Implementation may provide additional local encryption, but the product may claim it only when it is actually implemented and tested.

## 12.3 Local access is still security-relevant

Local-first does not mean “automatically secure.”

Anyone who can obtain direct access to the user's machine or project files may potentially access project data through the operating system or file mechanisms.

The product therefore must not falsely tell users that keeping a project local automatically makes it inaccessible to other people with access to the machine/files.

## 12.4 User responsibility boundary

OpenFrame controls application behavior.

The user or organization controls the physical/OS environment in which the local files are stored.

This distinction must remain explicit in documentation and security design.

---

# 13. Save, Autosave, Recovery, and Security

## 13.1 Autosave is not synchronization

Autosave persists local project changes.

It does not mean:

- cloud synchronization;
- remote publication;
- collaborator sharing;
- automatic external transmission.

## 13.2 Save status

The status labels:

- Saved;
- Saving;
- Save Error;
- Offline;
- collaboration/session state;
- pending package operation;

must describe local/application state accurately.

They must not imply cloud backup unless such a service is actually in use.

## 13.3 Recovery state is project-sensitive

Automatic History Points, Snapshots, and recovery state may contain the same sensitive project content as the main project.

They must therefore remain within the relevant local/project privacy boundary.

## 13.4 Recovery choices

After abnormal shutdown, the user may be offered:

- Recover latest state;
- Use last confirmed save;
- Dismiss recovery copy;
- Compare/review where supported.

Recovery must not silently replace the last confirmed project state.

## 13.5 Recovery and private notes

If a user's recoverable state contains Private Notes, another collaborator must not gain access to those notes merely because the project is reopened or a collaboration session begins.

## 13.6 Backup before risky operation

Where a risky migration/import can substantially change project state, the product should offer a backup first.

The backup should be recognized as a private project/recovery artifact, not a review package.

---

# 14. Deletion, Archive, Restore, and Permanent Removal

## 14.1 Recoverable deletion

The Domain/Data model includes Deleted Item.

Deletion therefore has a recovery boundary before permanent removal where supported.

## 14.2 Deletion does not mean immediate disappearance from every historical artifact

Historical Snapshots, Activity Entries, review records, and exported files are conceptually distinct from the current live object.

The application must not rewrite historical snapshots simply because a current object was later deleted.

## 14.3 Restore

Restore must use the Deleted Item metadata needed to return the object to its prior context where the FSD permits recovery.

## 14.4 Permanent deletion

The PRD and Domain/Data model distinguish archive from permanent deletion.

Permanent deletion requires deliberate user confirmation.

The application must not conflate:

```text
Archive
```

with:

```text
Permanent deletion
```

## 14.5 Security of deleted state

Recoverable deleted content should not be surfaced to users who do not have permission to access the underlying project state.

## 14.6 AI and deletion

AI cannot independently delete project content.

A user-requested AI deletion must pass:

```text
Request
→ Permission
→ State checks
→ Proposed Change Set
→ Preview
→ User acceptance
→ Normal application mutation
```

---

# 15. Activity History and Audit Boundary

## 15.1 Purpose

OpenFrame Activity History is a lightweight project-orientation and accountability mechanism.

It is not an enterprise SIEM, network packet log, or full keystroke recorder.

## 15.2 Meaningful events

Activity may record events such as:

- project created;
- draft created;
- draft locked;
- revision started;
- scene card moved;
- breakdown source changed;
- catalog item created;
- shooting day created;
- call sheet finalized;
- exchange package imported/exported;
- collaboration session started/ended;
- meaningful AI-assisted mutation;
- conflict resolution.

## 15.3 No keystroke surveillance

Ordinary typing must not become separate visible activity events.

This remains consistent with the PRD/FSD/UX/UI rule that Activity History is for meaningful events, not enterprise-style monitoring.

## 15.4 Activity fields

The Domain/Data model defines:

- `activity_id`;
- `project_id`;
- `actor_user_id` where applicable;
- `action_type`;
- target type/ID where relevant;
- timestamp;
- human-readable summary.

The AI specification further requires meaningful applied AI actions to identify:

- actor;
- AI-assisted origin;
- action;
- scope;
- time;
- result.

## 15.5 Audit versus transcript

AI history and Activity History are different concepts.

AI conversation history is not the same as project Activity.

The application must not turn every model token or internal reasoning step into an audit event.

## 15.6 Activity privacy

Activity visibility follows project activity visibility and user permissions.

The application should not use Activity History as a mechanism to expose private note content or restricted project information.

---

# 16. AI Security Boundary

## 16.1 System-wide AI, bounded by OpenFrame

The OpenFrame AI is a system-wide natural-language assistant and command interface.

It is not:

- a separate autonomous filmmaker;
- an independent database;
- an authoritative project source;
- a process with unrestricted filesystem access.

## 16.2 Canonical execution boundary

The required logical boundary is:

```text
AI
 ↓
Authorized Application Tools
 ↓
Canonical Domain/Data Layer
 ↓
Project
```

Not:

```text
AI
 ↓
Unrestricted project-file write
```

## 16.3 AI cannot access arbitrary filesystem state

The AI specification explicitly prohibits unrestricted filesystem/project-package access as a substitute for the OpenFrame object model.

This prevents the model from bypassing:

- permissions;
- private-note visibility;
- object identity;
- snapshots;
- Change Set validation;
- conflict rules;
- locked states;
- recovery controls.

## 16.4 AI inherits effective permissions

AI actions occur under the current user's effective permission.

Examples:

- Viewer cannot mutate;
- Commenter cannot mutate prohibited core content;
- Editor can mutate only permitted content;
- Owner can perform owner-only operations;
- AI cannot elevate a user.

## 16.5 AI respects state restrictions

AI cannot bypass:

- locked drafts;
- approved artifacts;
- stale documents;
- archived objects;
- deleted/recoverable-object rules;
- private-note protection;
- unavailable external files;
- collaboration conflicts.

## 16.6 AI mutation safety

The normative sequence is:

```text
User request
   ↓
Interpretation
   ↓
Scope resolution
   ↓
Permission/state checks
   ↓
Deterministic tool/application operation
   ↓
Proposed Change Set
   ↓
Preview
   ↓
Explicit user acceptance
   ↓
Normal application mutation
   ↓
Undo / Activity where supported
```

## 16.7 No authority delegation to model output

Model text is not permission.

A model saying:

> “The user wants me to change this.”

does not replace the application's authorization evaluation.

Likewise, model output cannot grant itself storage authority.

---

# 17. AI Request / Result Privacy

## 17.1 AI Request

The Domain/Data model supports AI Request with fields including:

- requesting user;
- project scope;
- session context;
- resolved scope;
- original request;
- interpreted intent;
- operation class;
- target objects;
- authorization state;
- external-processing state;
- model reference where applicable;
- status and timestamps.

These records can contain sensitive project information.

## 17.2 AI Result

AI Result may contain:

- natural-language response;
- structured data;
- provenance;
- suggested changes;
- status;
- error information.

It must not contain another user's restricted private information merely because the model could infer or retrieve it.

## 17.3 Provenance

When a response materially depends on project scope, source/provenance should identify enough context to let the user understand the result without exposing unauthorized data.

## 17.4 Exact facts

Exact project facts must come from deterministic application data/calculations.

This reduces both accuracy risk and privacy overreach because the AI does not need broad model-generated guesses about project state.

## 17.5 Conversation context

Conversation context must not silently expand permissions.

A safe follow-up request may reuse context only within the same effective access boundary.

---

# 18. Local AI Privacy

## 18.1 Local AI

Local AI is the preferred path for:

- private project work;
- offline operation;
- low-latency assistance where practical;
- users who do not want project content transmitted externally.

## 18.2 Local model boundary

The local model still does not become the project source of truth.

The model receives permitted context through the application and produces language, suggestions, or tool requests.

## 18.3 Offline AI

Configured local AI may remain available offline.

If local AI is unavailable, core OpenFrame workflows continue.

## 18.4 No false cloud indication

When local AI is used, the UI must not claim that project data is being sent to a cloud provider.

---

# 19. External AI Privacy

## 19.1 External AI is optional

External AI is a user-selected capability.

It is not required for core OpenFrame workflows.

## 19.2 Pre-transmission disclosure

Before a project-containing external AI request is transmitted, the user must see a disclosure indicating:

- that an external provider is being used;
- what context is being sent;
- what category of project information is included;
- configured provider/model identity where practical;
- that the user can cancel.

## 19.3 Disclosure must be truthful

The UI must not imply:

- local processing when the request is external;
- external processing when the request is local;
- OpenFrame ownership of third-party provider processing;
- provider retention guarantees that are not actually known.

## 19.4 Minimum necessary external context

Only authorized/requested context should be sent.

The product must not automatically send:

- the entire project;
- unrelated workspaces;
- hidden host-only data;
- another collaborator's Private Notes;
- unauthorized objects;
- unrelated project files.

## 19.5 External provider ownership boundary

Sending content to an external AI provider for a requested operation does not make that provider the owner of the OpenFrame project.

However, OpenFrame cannot truthfully promise how an external provider stores or retains transmitted information unless the relevant provider contract/policy is known and represented.

Therefore the product should disclose transmission without making unsupported claims about third-party data retention.

## 19.6 Cancellation

The user must be able to cancel the external request before transmission.

## 19.7 External AI mutation

Even when external AI provides the language/model processing:

- project mutation remains local;
- application permissions remain authoritative;
- Change Set rules remain authoritative;
- user acceptance remains required;
- the external model never receives direct project-write authority.

## 19.8 Network failure

If an external AI request cannot reach the provider:

- report the failure;
- do not treat the request as successfully processed;
- leave local project content unchanged;
- allow local AI/core workflows to continue where available.

---

# 20. AI During Local-Network Collaboration

## 20.1 Collaboration scope is inherited

AI receives no more data than the current participant is permitted to access in the active shared scope.

## 20.2 Host-only information

A participant's AI must not retrieve hidden host-only data.

## 20.3 Private Notes

A participant's AI must not retrieve another participant's Private Notes.

## 20.4 Shared Story-only session

If the host shares Story only:

- Story Board content may be available;
- unrelated screenplay/production content remains unavailable;
- private notes remain private.

## 20.5 Shared Screenplay session

If Screenplay is shared:

- screenplay access follows role;
- private notes remain private;
- lock/revision state remains enforced.

## 20.6 Shared Production session

If Production is shared:

- only permitted production content is exposed;
- unrelated private areas remain unavailable.

## 20.7 Concurrent AI Change Sets

If an AI Change Set was prepared against an earlier collaboration state, it must be revalidated before application.

Collaboration does not weaken stale-base protection.

---

# 21. Collaboration Privacy

## 21.1 Temporary LAN session

A Collaboration Session is temporary.

It is not permanent project ownership.

It is not permanent authorization.

It is not a cloud publication mechanism.

## 21.2 Session identity

The user interface should clearly identify:

- host;
- shared project;
- shared scope;
- participant;
- participant role;
- session state.

## 21.3 Presence

Presence is operational collaboration information.

It need not become a permanent Activity Entry for every transient presence event.

## 21.4 Soft locks

Soft locks are temporary collaboration state.

They do not create permanent project restrictions.

When a participant disconnects, the lock must not permanently lock the project area.

## 21.5 Session end

Ending a LAN session:

- stops the shared session;
- removes temporary shared-session context;
- does not delete the project;
- does not upload/publish the project to OpenFrame cloud;
- does not automatically rewrite normal project permissions.

## 21.6 Disconnection

Network failure or participant disconnect must not:

- silently delete changes;
- silently overwrite another user's data;
- expose private data;
- convert a temporary session into permanent sharing.

---

# 22. Exchange Package Privacy

## 22.1 Package scope is explicit

An Exchange Package contains an intentionally selected subset of a project.

The package may contain:

- selected workspace/page content;
- source version identity;
- required context;
- comments if selected;
- attachments if selected;
- package metadata.

## 22.2 Package does not grant project-wide access

Receiving a package does not grant access to the entire source project.

The receiver operates only within the package workflow.

## 22.3 Privacy review before export

The export UI should make the outgoing scope visible before package creation.

The user should be able to understand:

- what project/workspace is included;
- what version is included;
- what comments are included;
- what attachments are included;
- whether private content is excluded.

## 22.4 Ordinary review package

Private Notes remain excluded.

Unrelated project content remains excluded.

## 22.5 Response package

A response package carries only the information supported by the original package workflow and selected response content.

## 22.6 Stale packages

A stale package is not automatically trusted because the sender had older valid access.

It must still go through:

- source/version comparison;
- mapping;
- validation;
- conflict detection;
- user review where required.

## 22.7 Unmapped material

Unmapped comments or review material must be retained through the supported review queue/unmapped path rather than silently discarded.

---

# 23. Full Project Packages and Backup Privacy

## 23.1 Full Project Package

The Full Project Package is for moving a working OpenFrame project to another installation.

It may contain substantially more project information than an Exchange Package.

Therefore the export classification must be clear.

## 23.2 Backup Package

Backup is a private recovery-oriented artifact.

It should be treated as a project copy, not as a review package.

## 23.3 User warning boundary

Before creating a full-project or backup artifact, the user should understand that it may contain project information that would never appear in a normal review package.

## 23.4 No accidental downgrade

A user exporting a Full Project Package should not accidentally receive a review package that omits required project state.

Likewise, a user exporting a Review Package should not accidentally receive a full-project copy.

## 23.5 Package identity

Exchange Package, Full Project Package, and Backup Package must remain distinguishable concepts.

The Import/Export specification also aligns the PRD term “Project Archive” with the project-level portability concept represented as Full Project Package; this does not collapse it into a review exchange.

---

# 24. Document Export Privacy

## 24.1 Ordinary document exports are snapshots

A normal document export:

- reads an authoritative source;
- applies explicit scope;
- creates an outgoing snapshot;
- does not mutate project content.

## 24.2 Source-of-truth security

Examples:

| Output | Authoritative source |
|---|---|
| Screenplay PDF | Selected Screenplay Draft |
| FDX | Selected Screenplay Draft |
| Fountain | Selected Screenplay Draft |
| DOCX screenplay | Selected Screenplay Draft |
| Story Board PDF | Current Story Board |
| Breakdown PDF | Breakdown + selected Production Source context |
| Catalog export | Production Catalog |
| Shot List PDF | Shot List |
| Storyboard export | Storyboard |
| Schedule export | Shooting Schedule |
| Call Sheet PDF | Call Sheet document |
| Full Project Package | Entire supported project state |
| Exchange Package | Explicitly selected workspace/project scope |

Security must follow the selected source and scope rather than exposing unrelated content.

## 24.3 Private notes

Private Notes are excluded from standard screenplay/output flows.

## 24.4 Comments

Comments are included only when the selected export workflow supports them and the user explicitly includes them.

Normal screenplay output does not silently insert internal review comments.

## 24.5 External files

Ordinary document exports do not automatically embed unrelated external files.

---

# 25. Import Security and Data Integrity

## 25.1 Import is a data-ingestion boundary

Supported screenplay inputs include:

- PDF;
- FDX;
- Fountain;
- TXT;
- DOCX;
- pasted screenplay text.

These inputs must be treated as incoming data, not as project authority.

## 25.2 Parse before mutation

The import lifecycle is:

```text
Select source
→ Parse
→ Preview
→ Warnings
→ Choose destination
→ Import
→ Complete
```

## 25.3 Existing project protection

Screenplay import does not overwrite the current draft by default.

## 25.4 Uncertain parsing

If structure is uncertain:

- show warnings;
- show the best interpretation available;
- allow the user to inspect before applying;
- do not present uncertain results as certain.

## 25.5 Project integrity on failure

If import fails, the existing project must remain unchanged.

## 25.6 Full project/package imports

Before risky full-project or exchange imports into an existing project, the application should offer backup where the FSD requires/permits that behavior.

## 25.7 Mapping and identity protection

Import must preserve stable identities when a package clearly represents an existing object.

It must create new identities when the imported material represents a copy.

It must not accidentally merge:

```text
New copy
```

into:

```text
Existing canonical object
```

## 25.8 Validation before apply

Validation happens before mutation.

The Import Session holds:

- incoming package;
- target project;
- base snapshot;
- preview state;
- conflicts;
- result state.

The application must not use an incoming package as a direct write-through mechanism.

## 25.9 No blind overwrite

Conflicting import does not silently destroy host data.

## 25.10 Missing external references

Missing external references appear in the import report.

The application does not silently replace them.

## 25.11 Imported content remains data

Imported text does not become an OpenFrame command simply because it contains words resembling commands or instructions.

The import layer's job is to interpret supported document structure, not to execute arbitrary imported content as application operations.

---

# 26. Change Sets as a Security Boundary

## 26.1 Change Set meaning

A Change Set is a bounded collection of changes produced by:

- user edit;
- review response;
- package import;
- approved AI mutation.

## 26.2 Required provenance

A Change Set identifies:

- origin;
- responsible/requesting user;
- approver where applicable;
- target objects;
- operations;
- affected modules;
- base version;
- review state;
- validation state.

## 26.3 Approval is not permission

The Domain/Data model explicitly states:

> Approval does not override permission restrictions.

Therefore:

```text
User approval
      ≠
Permission grant
```

Both must be valid.

## 26.4 Stale protection

If project state changes after a Change Set is prepared:

- the Change Set becomes Stale/Conflict as appropriate;
- it must be revalidated;
- a new preview may be required.

## 26.5 Rejected/invalid changes

Rejected, invalid, stale, or conflicted Change Sets do not mutate the project.

---

# 27. Locked Drafts and Approved State

## 27.1 Locked screenplay

A locked draft is a formal project state.

Security must preserve the lock.

## 27.2 AI cannot bypass lock

An AI request cannot silently edit a locked draft.

Normal post-lock revision behavior applies.

## 27.3 Collaboration cannot bypass lock

A LAN participant cannot use shared-session access as a bypass around screenplay lock.

## 27.4 Import cannot be a lock bypass

Importing a package into a locked area must follow the same state/permission rules as manual edits.

## 27.5 Export is not mutation

Exporting a locked draft is a read/snapshot operation and does not unlock or mutate the source.

---

# 28. Data Provenance and Historical Integrity

## 28.1 Snapshot rule

A Snapshot is an immutable point-in-time representation used for:

- export;
- review;
- revision;
- call-sheet finalization;
- recovery;
- comparison.

## 28.2 Historical immutability

A later project edit must not silently rewrite the meaning of an earlier snapshot.

Examples:

- an issued call sheet remains historical;
- an exported screenplay remains the historical export artifact;
- an Exchange Package remains its source snapshot;
- a recovery point remains the recovery state it captured.

## 28.3 Provenance for downstream production

Breakdown, Shot List, Storyboard, Schedule, Call Sheet, and Sides may depend on screenplay source/version context.

That provenance is part of integrity and security because it prevents a user from unknowingly treating unrelated or stale content as if it were the same source.

---

# 29. Cross-Project and Episodic Privacy

## 29.1 Project boundary

A user's project files may coexist on the same machine.

That does not merge permissions.

## 29.2 Episode boundary

For episodic/series work, AI and data queries must preserve the requested episode/season/series scope.

## 29.3 No accidental cross-project search

A project-scoped request must not pull another project's data merely because:

- names match;
- objects have similar text;
- the same user owns both projects;
- both projects are open/recent.

## 29.4 Global Idea Vault

The Global Idea Vault is intentionally cross-project at the product level.

This does not mean every project collaborator automatically gains access to every global item.

Any future shared/global permission model must remain explicit.

---

# 30. Collaboration Permission Lifetime

## 30.1 Temporary session access

LAN access is granted for the active collaboration session context.

## 30.2 Session end

When the session ends, temporary shared-session context ends.

## 30.3 Normal project permissions remain separate

Ending a session does not silently:

- delete the user;
- modify normal project permissions;
- upload the project to cloud;
- transfer ownership.

## 30.4 Exchange Package scope

Remote collaboration through an Exchange Package is governed by the contents and workflow of that package.

The package is not a permanent project ACL.

---

# 31. Local-Network Security Boundary

## 31.1 Product requirement

Only participants with granted access should participate in the shared project scope.

## 31.2 Shared-scope enforcement

Host-selected scope must control what participants can access.

## 31.3 Role enforcement

The participant's granted role must remain visible and enforceable.

## 31.4 Temporary state

Presence and locks remain session-local.

## 31.5 Unauthorized network participant

An unauthorized network peer must not obtain project content merely because the project exists on a machine participating in a LAN session.

## 31.6 Technical transport is an implementation boundary

The current product documents do not prescribe:

- a specific LAN protocol;
- a specific transport encryption scheme;
- a specific identity provider;
- a specific pairing mechanism.

Those choices are implementation responsibilities and must satisfy the product-level outcome:

> **Only authorized participants receive the permitted project scope, and transport failure must not become data corruption or unauthorized access.**

---

# 32. Privacy of Notifications, Status, and UI

## 32.1 Status indicators

The UI may show:

- Saved;
- Saving;
- Save Error;
- Offline;
- Local Collaboration Session;
- External AI connection state;
- pending import/export/package state.

## 32.2 Avoid sensitive leakage

Status messages should not expose:

- Private Note content;
- hidden collaboration content;
- unauthorized filenames/objects when the user cannot access them;
- external AI prompt contents to other users.

## 32.3 Permission screen

The UX must show:

- current role;
- granted access scope;
- private-area indication where relevant.

This matches the UX/UI specification.

## 32.4 External AI disclosure surface

External AI data transmission must be obvious enough that a user does not mistake it for local processing.

## 32.5 Recovery UI

Recovery UI should communicate that a recovery state exists without unnecessarily exposing protected content outside the user's permission boundary.

---

# 33. Privacy in Printing and Exported Documents

## 33.1 Printed output

Printing uses the same document-generation logic as PDF generation.

Therefore privacy rules for normal export apply to print output.

## 33.2 Private Notes

Private Notes must not silently appear in printed screenplay or call-sheet documents.

## 33.3 Review comments

Internal comments are not automatically part of polished screenplay output.

## 33.4 Selected scope

Printed output must correspond to the user's selected scope rather than silently printing unrelated project content.

---

# 34. AI Privacy Failure Modes

The following are prohibited.

## 34.1 Private-note inference leak

Bad:

> “I cannot show the note, but the writer is planning to kill the producer.”

The denial itself leaks protected information.

Correct behavior:

> The requested information is private and cannot be accessed in this context.

## 34.2 Cross-project leak

Bad:

> “Your other film has a similar scene and here is the draft.”

The second project was not in scope.

## 34.3 Host-only leak

Bad:

> “The host has marked Scene 18 as secretly unavailable.”

That exposes hidden host information.

## 34.4 Permission bypass through natural language

Bad:

> “I know you are a Viewer, but the owner probably intended you to change this.”

The application permission remains authoritative.

## 34.5 Permission bypass through tool arguments

Model-generated tool parameters cannot elevate authorization.

## 34.6 Stale-change application

Bad:

> AI prepared a Change Set, project changed, AI applies the old proposal anyway.

Correct:

> Revalidate stale state and present a new/updated review path.

---

# 35. External AI Data-Disclosure UX Contract

The disclosure surface should answer five questions quickly:

```text
What is leaving OpenFrame?
Where is it going?
Why is it being sent?
What project context is included?
Can I cancel?
```

Minimum disclosure content:

- external provider indication;
- provider/model where practical;
- selected scope;
- category of data being sent;
- cancel action.

Example:

```text
External AI

Provider: [Configured Provider]
Context: Current Scene + selected screenplay text
Data sent: Scene heading, action/dialogue text, selected comments
Private notes: Excluded

[Cancel] [Send to External AI]
```

This is an illustrative UI contract, not a required visual design.

---

# 36. Privacy-Preserving AI Scope Examples

## Example A — Current scene question

User:

> “Who appears in this scene?”

Permitted context:

- current screenplay scene;
- accessible character references required for the answer.

Not automatically included:

- unrelated scenes;
- private notes;
- unrelated projects.

## Example B — Project statistics

User:

> “How many scenes are in this draft?”

Use deterministic canonical project data.

Do not send the entire script to an external model merely to calculate an exact count.

## Example C — Break-down suggestion

User:

> “Suggest props for this scene.”

External transmission, if used, should be limited to the scene/project context required for the suggestion and must be disclosed.

## Example D — Private-note request

User:

> “Read the writer's private notes and tell me what they think.”

Denied unless the current user is the authorized owner/private context.

---

# 37. Import / Export Privacy Matrix

| Operation | Default privacy boundary | Private Notes |
|---|---|---|
| Screenplay PDF | Selected draft/scope | Excluded |
| FDX | Selected draft | Excluded |
| Fountain | Selected draft | Excluded |
| DOCX screenplay | Selected draft | Excluded |
| Story Board PDF | Current Story Board scope | Excluded |
| Breakdown export | Breakdown/source scope | Excluded |
| Schedule export | Schedule scope | Excluded |
| Call Sheet PDF | Call Sheet scope | Excluded |
| Review Exchange Package | Explicitly selected review scope | Excluded |
| Full Project Package | Entire supported project state | May be included where supported for private transfer |
| Backup Package | Recovery/project state | May be included where supported |
| LAN shared Story scope | Host-selected shared scope | Private |
| LAN shared Screenplay scope | Permitted screenplay scope | Private |
| LAN shared Production scope | Permitted production scope | Private |
| External AI | Explicitly selected/requested context | Never another user's private notes |

---

# 38. Offline Privacy Rules

## 38.1 Offline is normal

Offline is not an error.

## 38.2 Offline does not reduce privacy

The absence of internet must not cause the application to:

- upload data elsewhere;
- create a cloud copy;
- downgrade private-note protection;
- disable local permission checks.

## 38.3 Local AI offline

Configured local AI may work offline with the same permission boundary as online local AI.

## 38.4 External AI offline

External AI is unavailable if the provider cannot be reached.

Core project work continues.

## 38.5 No sync queue

OpenFrame does not require a cloud synchronization queue for ordinary local project persistence.

A “sync pending” state must not become an unauthorized storage side channel.

---

# 39. Crash and Failure Security

## 39.1 Save failure

A failed save must not be reported as successful.

## 39.2 Network failure

Network failure must not:

- delete project content;
- silently fork the project;
- convert a session into a new project;
- change ownership;
- expose hidden content.

## 39.3 AI failure

AI failure leaves local project content unchanged.

## 39.4 Import failure

Import failure leaves the destination project unchanged.

## 39.5 Conflict-resolution failure

If a conflict cannot be safely resolved:

- preserve both sides where possible;
- keep the conflict visible;
- do not choose a winner silently.

## 39.6 Participant join failure

If a participant cannot join:

- no local project mutation occurs;
- explain the failure;
- allow normal local work.

---

# 40. Data Ownership in a Multi-User Project

## 40.1 One project, multiple permissions

A single project can be worked on by multiple users while remaining a single logical project.

## 40.2 User contributions

Comments, activity attribution, AI requests, Change Sets, and private notes remain associated with the relevant user identities where the Domain/Data model supports it.

## 40.3 Private does not mean invisible to the owner

The owner's Private Notes remain part of the user's project context.

## 40.4 Shared content

When a user deliberately turns information into ordinary shared project content, that information follows the ordinary project permission model.

## 40.5 Removing a collaborator

Removal of collaboration access changes future authorized access to the project/session.

It does not retroactively recall:

- exported PDFs;
- package files;
- screenshots;
- copied text;
- other externally transferred material.

This reflects the basic distinction between changing OpenFrame access and controlling a copy that has already left the project boundary.

---

# 41. Security and Source-of-Truth Rules

Security controls must never collapse distinct authoritative sources.

Canonical sources remain:

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

A security feature must not create a second writable copy of any of these sources merely to enforce access control.

---

# 42. Security and Derived Data

Derived information may be recalculated from canonical data.

Examples:

- scene numbers;
- exact scene counts;
- schedule summaries;
- project statistics;
- relationship queries.

Security evaluation must occur against the underlying accessible source data.

A user must not gain unauthorized data simply because a derived summary was generated from a broader hidden scope.

---

# 43. Data Ownership and AI Provenance

AI output is not project authority.

The AI specification makes the distinction:

```text
AI interpretation
      ↓
Application query / tool
      ↓
Deterministic result
```

or:

```text
AI suggestion
      ↓
Change Set
      ↓
User acceptance
      ↓
Normal project data
```

Only the final accepted application mutation becomes project state.

The model itself never becomes an alternate project owner or database.

---

# 44. Data Retention Boundaries

## 44.1 OpenFrame local project retention

The core project remains available according to local project storage, backup, recovery, archive, and deletion behavior.

## 44.2 Activity retention

Activity History is meaningful project history, not a full surveillance log.

## 44.3 AI conversation retention

The FSD states that AI conversation history may be retained according to product settings.

The current baseline does not define a universal retention period or a separate enterprise retention system.

Therefore the implementation must not invent a fixed retention promise in product documentation unless separately specified.

## 44.4 External provider retention

External provider retention is outside the control of OpenFrame unless an explicit provider contract establishes otherwise.

The product should disclose transmission and configured provider identity where practical without making unsupported guarantees about third-party retention.

---

# 45. Security Documentation Truthfulness

OpenFrame product documentation, UI, and settings must distinguish:

```text
Implemented security control
```

from:

```text
User/environment responsibility
```

and:

```text
Third-party provider policy
```

Examples:

### OpenFrame controls

- permission enforcement;
- private-note boundary;
- scope selection;
- AI approval;
- no blind overwrite;
- external AI disclosure;
- offline behavior;
- local project persistence rules.

### User environment controls

- physical machine access;
- operating-system file permissions;
- storage-device custody;
- who receives exported files.

### External-provider controls

- third-party model processing;
- provider-side retention;
- provider-side account/data policies.

This separation prevents misleading security claims.

---

# 46. Security-Relevant Activity Events

Where Activity History is supported, the following meaningful security/privacy events may be recorded:

| Event | Why it matters |
|---|---|
| Permission changed | Access-control change |
| Collaborator added/removed | Shared-access change |
| Collaboration session started | Temporary access context |
| Collaboration session ended | Temporary access context closed |
| Exchange Package exported | Data left current project boundary |
| Exchange Package imported | External content entered project |
| Full Project Package exported | Large project-data copy created |
| Backup created | Recovery copy created |
| AI external request invoked | Project context sent externally |
| AI mutation applied | User-approved AI-originated change |
| Conflict resolved | Shared-state reconciliation |
| Project archived/restored | Lifecycle state change |
| Permanent deletion confirmed | Irreversible lifecycle operation |

These are meaningful activity events, not a requirement to log every internal operation.

---

# 47. Security Invariants by Data Path

## 47.1 Local editing

```text
User
→ authorized project
→ normal application mutation
→ local persistence
```

Invariant:

> No external transmission is required.

## 47.2 Local AI

```text
User
→ permitted project context
→ local model
→ application tools/results
```

Invariant:

> No external provider is involved unless explicitly selected.

## 47.3 External AI

```text
User
→ selected context
→ disclosure
→ user action
→ external provider
→ answer/suggestion
```

Invariant:

> Only the authorized/requested context is sent and the user is informed beforehand.

## 47.4 Exchange Package

```text
User
→ explicit package scope
→ snapshot
→ file transfer
```

Invariant:

> The package is a snapshot, not live project access.

## 47.5 LAN Collaboration

```text
Host
→ shared project/scope
→ participant admission
→ role-limited access
→ temporary shared session
```

Invariant:

> Session access is temporary and scope-limited.

## 47.6 Import

```text
Incoming package
→ validation
→ preview
→ identity/mapping
→ conflict detection
→ Change Set
→ user/application decision
```

Invariant:

> Incoming data is never trusted as direct authority over existing project state.

---

# 48. Threat / Failure Scenario Matrix

| Scenario | Required behavior |
|---|---|
| Internet unavailable | Core project work continues |
| External AI unavailable | AI request fails safely; project unchanged |
| User asks AI for unauthorized private note | Deny without protected-content disclosure |
| Viewer asks AI to edit screenplay | Deny mutation |
| AI proposes stale change | Revalidate before apply |
| External AI request includes unrelated project data | Prevent overbroad scope |
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

---

# 49. What This Specification Intentionally Does Not Define

To remain consistent with the current OpenFrame product scope, this document does not invent a mandatory technical architecture for:

- cloud identity management;
- OpenFrame account registration;
- enterprise SSO;
- password policy;
- MFA;
- OAuth/OIDC;
- certificate management;
- specific LAN cryptographic protocols;
- specific encryption algorithms;
- hardware security modules;
- centralized SIEM;
- server-side telemetry;
- enterprise DLP;
- legal/compliance certification;
- third-party provider contractual retention terms;
- biometric authentication;
- DRM.

Those may be future engineering or product decisions.

They must not be presented as existing OpenFrame features unless separately specified, implemented, and tested.

---

# 50. Implementation-Neutral Security Requirements

The following are product-level requirements, not technology mandates.

### SEC-001 — Local project ownership
The local project remains the primary user-owned project state.

### SEC-002 — No mandatory OpenFrame cloud
Core project use does not require OpenFrame cloud sign-in or synchronization.

### SEC-003 — Permission inheritance
Permissions apply consistently across local use, exchange, LAN collaboration, AI, import, and export.

### SEC-004 — Canonical role set
The access-control role set remains Owner, Editor, Commenter, Viewer, and Export-only.

### SEC-005 — Owner control
Owner-only project operations are restricted to authorized owners.

### SEC-006 — Editor boundary
Editors can edit only authorized project content.

### SEC-007 — Commenter boundary
Commenters can review/comment within allowed scope but cannot use adjacent workflows to bypass core edit restrictions.

### SEC-008 — Viewer boundary
Viewers are read-only.

### SEC-009 — Export-only boundary
Export-only users cannot enter live editing sessions.

### SEC-010 — Private-note isolation
Private Notes remain owner/private-context only.

### SEC-011 — Private-note search isolation
Unauthorized users cannot discover another user's Private Notes through search or AI retrieval.

### SEC-012 — No hidden cross-project access
Project-scoped queries remain within project boundaries.

### SEC-013 — Episodic scope isolation
Episode/season/series queries preserve explicit scope.

### SEC-014 — AI permission inheritance
AI cannot exceed the current user's effective permissions.

### SEC-015 — AI tool boundary
AI cannot directly write project storage.

### SEC-016 — AI mutation preview
Project-changing AI operations require visible preview.

### SEC-017 — AI mutation acceptance
Project-changing AI operations require explicit user acceptance.

### SEC-018 — AI stale protection
Stale Change Sets cannot apply without revalidation.

### SEC-019 — AI private-note protection
AI cannot expose another user's Private Notes.

### SEC-020 — External AI disclosure
External AI use is disclosed before transmission.

### SEC-021 — External AI scope minimization
Only authorized/requested context is sent externally.

### SEC-022 — External AI cancellation
The user can cancel before external transmission.

### SEC-023 — External AI no-write authority
External models never receive direct project-write authority.

### SEC-024 — Local AI truthfulness
Local AI is clearly represented as local.

### SEC-025 — External AI truthfulness
External AI is clearly represented as external.

### SEC-026 — Export snapshot isolation
Ordinary export never mutates source project state.

### SEC-027 — Private-note export exclusion
Private Notes are excluded from normal exports.

### SEC-028 — Review-package privacy
Private Notes are excluded from ordinary review exchange packages.

### SEC-029 — Full-project privacy disclosure
Full Project Packages clearly identify their broader project-data scope.

### SEC-030 — Backup privacy
Backup artifacts are treated as private recovery/project copies, not ordinary review packages.

### SEC-031 — Package scope
Exchange Packages contain explicitly selected project scope.

### SEC-032 — Package non-authority
An Exchange Package does not grant project-wide access.

### SEC-033 — Import validation
Validation occurs before import mutation.

### SEC-034 — Import no-blind-overwrite
Conflicting imports do not silently destroy host state.

### SEC-035 — Identity-safe import
Import distinguishes existing-object changes from new copies.

### SEC-036 — Missing-reference reporting
Missing external references are reported.

### SEC-037 — Full-project collision safety
Project identity collision offers preservation-oriented choices.

### SEC-038 — Recovery privacy
Recovery state follows the same relevant project/privacy boundary as the source project.

### SEC-039 — Deleted-state privacy
Recoverable deleted objects remain permission-scoped.

### SEC-040 — Historical snapshot integrity
Historical snapshots are immutable representations of their captured state.

### SEC-041 — Activity boundary
Activity History records meaningful events, not every keystroke.

### SEC-042 — Activity privacy
Activity visibility follows project/activity permissions.

### SEC-043 — LAN role enforcement
LAN participants receive only their granted role and shared scope.

### SEC-044 — LAN temporary authority
Collaboration session state is temporary and does not itself redefine permanent project ownership.

### SEC-045 — LAN privacy
Hidden host-only and private participant data remain protected.

### SEC-046 — LAN disconnect safety
Disconnect does not silently destroy or publish project data.

### SEC-047 — LAN conflict visibility
Same-object conflicts remain visible and reviewable.

### SEC-048 — AI collaboration scope
AI during collaboration operates within participant permissions and shared scope.

### SEC-049 — Offline privacy parity
Offline state does not weaken permission/privacy rules.

### SEC-050 — No false security claims
OpenFrame does not claim unimplemented encryption, synchronization, backup, retention, or provider controls.

### SEC-051 — External storage integrity
External-drive loss does not create a silent divergent project.

### SEC-052 — Save truthfulness
Save status reflects actual local persistence.

### SEC-053 — Failure safety
AI, import, network, and save failures do not silently corrupt or replace the last safe local state.

### SEC-054 — Permission denial integrity
Denied operations do not mutate project state.

### SEC-055 — Lock enforcement
Security boundaries respect locked screenplay state.

### SEC-056 — Approval does not grant permission
User approval cannot override role/object restrictions.

### SEC-057 — Data-minimized network use
Non-project network features must not implicitly transmit project content.

### SEC-058 — Project ownership clarity
The UI and documentation consistently communicate that the project is the filmmaker's work.

### SEC-059 — Transport/security separation
Technical transport mechanisms are implementation choices but must satisfy product-level authorization and confidentiality outcomes.

### SEC-060 — Provider-boundary honesty
External provider behavior is described only to the extent that OpenFrame actually knows and controls it.

---

# 51. Traceability to Existing PRD Requirements

This specification does not create new PRD scope. It specializes existing requirements.

| PRD ID | Security / privacy relevance |
|---|---|
| PRD-CORE-001 | Project lifecycle, user context, ownership |
| PRD-CORE-002 | Delete, undo, search, local interaction boundary |
| PRD-CORE-003 | Project status and errors |
| PRD-SCRIPT-003 | Review, comments, Private Notes |
| PRD-SCRIPT-004 | Lock/revision integrity |
| PRD-SCRIPT-005 | Import safety |
| PRD-SCRIPT-006 | Export snapshots and privacy |
| PRD-COL-001 | Permissions |
| PRD-COL-002 | Review Packages |
| PRD-COL-003 | Page-specific Exchange Packages |
| PRD-COL-004 | Better exchange controls |
| PRD-COL-005 | LAN collaboration |
| PRD-COL-006 | Collaboration conflict handling |
| PRD-OFF-001 | Offline ownership, saving, portability, external storage, backup |
| PRD-AI-001 | AI assistant, disclosure, permission, user control |
| PRD-AI-002 | AI-oriented supporting behavior where applicable |

---

# 52. Traceability to Existing FSD Requirements

The principal FSD requirements specialized here are:

| FSD ID | Security / privacy relevance |
|---|---|
| FSD-SCRIPT-020 | Private/working scene notes excluded from standard screenplay output |
| FSD-SCRIPT-033 | Private Note not visible to unauthorized users |
| FSD-SCRIPT-038..044 | Screenplay import and import safety |
| FSD-SCRIPT-045..050 | Screenplay exports and snapshot behavior |
| FSD-COL-001..005 | Roles and permissions |
| FSD-COL-006..016 | Exchange package scope, preview, staleness, conflict and mapping |
| FSD-COL-017..022 | LAN session, role, presence, conflict, disconnect, end |
| FSD-OFF-001..010 | Offline operation, save, recovery, backup, portability, external storage |
| FSD-AI-001 | AI optional |
| FSD-AI-011 | External AI disclosure |
| FSD-AI-012 | AI failure safety |
| FSD-AI-019 | Permission inheritance |
| FSD-AI-020 | Locked/private/approval enforcement |
| FSD-AI-021 | Change Set base-version validation |
| FSD-AI-022 | Application tool boundary and validation |
| FSD-AI-024 | Scope/provenance disclosure |
| FSD-AI-025 | AI lifecycle/activity integration |
| FSD-AI-027 | Cross-project/episodic scope control |

---

# 53. Traceability to Current Specialized Specifications

## 53.1 Import / Export

The Security specification relies on and preserves:

- four distinct portability concepts;
- ordinary document snapshots;
- Exchange Packages;
- Full Project Packages;
- Backup Packages;
- private-note exclusion for normal outputs;
- explicit package scope;
- validation before mutation;
- stale and conflict handling;
- identity mapping;
- missing external-reference reporting.

The Import/Export specification states that its privacy contract excludes Private Notes from normal review exchange, permits supported private data within explicitly understood full-project/private-transfer or backup contexts, and requires AI not to bypass validation or approval.

## 53.2 Offline / Collaboration

The Security specification relies on and preserves:

- Solo Local;
- Exchange Collaboration;
- Local-Network Collaboration;
- local project ownership;
- no mandatory OpenFrame cloud;
- role enforcement;
- private-note protection;
- host-selected scope;
- temporary session state;
- presence/soft locks;
- visible conflicts;
- disconnect safety;
- Exchange Package fallback;
- AI privacy during collaboration.

---

# 54. Traceability to Domain/Data Concepts

The principal canonical domain objects involved are:

- Application User;
- Project;
- Project Settings;
- Project File;
- Screenplay Draft;
- Screenplay Scene;
- Comment;
- Private Note;
- Activity Entry;
- Exchange Package;
- Import Session;
- Collaboration Session;
- AI Request;
- AI Result;
- AI Tool Invocation;
- Change Set;
- Snapshot;
- Deleted Item.

The security model does not introduce a separate parallel ownership database.

## 54.1 Ownership-related domain fields

Especially relevant fields include:

- `Project.owner_user_id`;
- `Application User.user_id`;
- `Application User.roles`;
- `Application User.permissions`;
- `Private Note.owner_user_id`;
- `Comment.author_user_id`;
- `Activity Entry.actor_user_id`;
- `AI Request.user_id`;
- `AI Request.authorization_state`;
- `AI Request.external_processing_state`;
- `Change Set.requesting_user_id`;
- `Change Set.approver_user_id`;
- `Change Set.base_version`;
- `Exchange Package.source_project_id`;
- `Exchange Package.source_object_ids`;
- `Import Session.target_project_id`;
- `Collaboration Session.host_user_id`.

---

# 55. UX/UI Security Contract

Security should be visible without turning OpenFrame into an enterprise administration product.

## 55.1 Required lightweight surfaces

The UX should provide:

- current project identity;
- current role/access;
- private-note indicators;
- external AI disclosure;
- offline/session status;
- package/export scope preview;
- import preview;
- conflict review;
- recovery decision;
- destructive-action confirmation.

## 55.2 Do not create an enterprise security dashboard

The product philosophy explicitly avoids enterprise administration complexity.

Security should be integrated into the workflows where it matters.

## 55.3 Plain-language security

Prefer:

> Private note — visible only to you

over:

> ACL-protected private annotation object

Prefer:

> This will send the selected screenplay text to an external AI provider.

over:

> External data egress event detected.

The underlying security model may be sophisticated; the user-facing language should remain understandable.

---

# 56. Security Rules for Common Workflows

## 56.1 Writing

```text
User writes
→ local project state
→ autosave
```

No external transmission is required.

## 56.2 Review

```text
User selects draft
→ adds comments
→ reviewers see permitted content
```

Private Notes remain private.

## 56.3 Export

```text
User selects scope
→ export snapshot
```

Private Notes excluded from normal output.

## 56.4 Review exchange

```text
User selects package scope
→ preview contents
→ export package
```

No project-wide access is granted.

## 56.5 LAN collaboration

```text
Owner starts session
→ selects scope
→ grants roles
→ participants join
→ temporary shared session
```

Private Notes remain private.

## 56.6 AI question

```text
User asks
→ scope resolved
→ permission check
→ deterministic retrieval where possible
→ answer
```

No mutation.

## 56.7 AI mutation

```text
User asks
→ permission/state checks
→ Change Set
→ preview
→ accept
→ normal application mutation
```

## 56.8 External AI

```text
Select context
→ disclose external processing
→ user confirms transmission
→ external answer
```

Project mutation still remains local.

## 56.9 Risky import

```text
Optional backup
→ parse
→ preview
→ mapping
→ conflict validation
→ apply selected changes
```

---

# 57. Privacy Rules for Project History and Revisions

## 57.1 Draft lineage

Named screenplay drafts are persistent version identities.

Security must preserve draft boundaries.

## 57.2 Automatic History Points

Automatic History Points are recovery history, not separate user-facing deliverables.

They remain project-sensitive.

## 57.3 Review history

Comments and review rounds may persist as supported history.

Review visibility follows the relevant project permission.

## 57.4 Locked draft and revisions

Post-lock revisions follow explicit production revision behavior.

Security cannot turn historical shooting scripts into silently editable shared documents.

---

# 58. Privacy and Production Data

Production planning may contain:

- locations;
- cast/crew information;
- props;
- wardrobe;
- vehicles;
- schedules;
- call sheets;
- shots;
- storyboards.

The current baseline does not define a separate enterprise personal-data taxonomy for these objects.

The security rule is therefore:

> Production content remains protected by the same project role and scope model rather than receiving an invented secondary identity system.

When a production object is exported or sent to external AI, only the selected/required context should be included.

---

# 59. Privacy and Call Sheets

Call Sheets are production documents and must follow their source/snapshot rules.

Private Notes do not appear in standard call-sheet output.

A call sheet may be exported as a historical snapshot.

Changing the later schedule does not silently rewrite a previously issued snapshot.

---

# 60. Privacy and Sides

Sides are script excerpts from a selected source/draft.

Security requires:

- correct source selection;
- correct scope;
- no accidental inclusion of unrelated private notes;
- no mutation of the source screenplay merely by generating sides.

---

# 61. Privacy and Reports

Reports are derived outputs.

A report must be generated from data within the user's authorized scope.

A report must not become a covert cross-module data dump simply because several underlying objects exist.

---

# 62. Exported Copies and Control Boundaries

Once the user deliberately exports a file/package outside OpenFrame, the application cannot guarantee control over every later copy.

Therefore the product must distinguish:

```text
OpenFrame access control
```

from:

```text
Control over copies already exported
```

The correct product behavior is to make outgoing scope clear before export and avoid accidental disclosure.

It is not to promise impossible revocation of files that have already been copied elsewhere.

---

# 63. Data Ownership Principles for External Providers

When external AI is used:

1. OpenFrame controls whether the request is sent.
2. OpenFrame controls the authorized/requested project context selected for that request.
3. OpenFrame must disclose that external processing occurs.
4. The external provider processes the transmitted information under its own service environment and policies.
5. OpenFrame must not claim third-party retention/deletion behavior that it does not control or know.
6. The provider does not become the canonical OpenFrame project database.
7. The provider does not receive direct mutation authority over the local project.

---

# 64. Security Acceptance Criteria

### SEC-AC-001 — Local ownership
A project can be created, opened, edited, saved, backed up, imported, exported, and recovered without OpenFrame cloud.

### SEC-AC-002 — Permission enforcement
A Viewer cannot perform an operation that requires edit permission.

### SEC-AC-003 — Commenter enforcement
A Commenter cannot use AI, import, or another workflow to bypass core edit restrictions.

### SEC-AC-004 — Export-only enforcement
An Export-only participant can create allowed snapshots/packages but cannot enter live editing.

### SEC-AC-005 — Private note isolation
Another collaborator cannot open or retrieve a user's Private Notes.

### SEC-AC-006 — Private note export exclusion
Normal screenplay export and review exchange do not include Private Notes.

### SEC-AC-007 — Private note AI protection
AI refuses unauthorized requests for another user's Private Notes without revealing protected detail.

### SEC-AC-008 — Search privacy
Global/project search does not reveal data outside the user's effective project scope.

### SEC-AC-009 — Cross-project isolation
A project-scoped query does not return another project's content.

### SEC-AC-010 — Episode scope
An episode-scoped query does not silently expand to unrelated episodes/series/projects.

### SEC-AC-011 — AI permission inheritance
AI receives the same effective permissions as the current user.

### SEC-AC-012 — AI direct-write prohibition
AI cannot write project storage without going through authorized application tools.

### SEC-AC-013 — AI preview
Every supported project-changing AI operation shows a proposed result before application.

### SEC-AC-014 — AI acceptance
Rejected AI proposals produce no project mutation.

### SEC-AC-015 — AI stale protection
A stale AI Change Set is revalidated before application.

### SEC-AC-016 — Locked draft protection
AI, collaboration, and import do not silently bypass screenplay lock.

### SEC-AC-017 — External AI disclosure
The UI shows external-provider disclosure before project data is transmitted.

### SEC-AC-018 — External AI cancellation
The user can cancel an external request before transmission.

### SEC-AC-019 — External AI minimization
External AI requests contain only authorized/requested context.

### SEC-AC-020 — No private-note transmission
Another user's Private Notes are never sent to an external AI provider by normal OpenFrame behavior.

### SEC-AC-021 — Local AI truthfulness
When local AI is used, the UI does not imply external transmission.

### SEC-AC-022 — AI failure safety
Failure to reach external AI leaves local project state unchanged.

### SEC-AC-023 — Exchange scope
An Exchange Package contains only the selected supported scope and context.

### SEC-AC-024 — Package no-access escalation
Receiving an Exchange Package does not grant project-wide access.

### SEC-AC-025 — Package privacy
Normal review packages exclude Private Notes.

### SEC-AC-026 — Full-project classification
Full Project Package export clearly differs from review-package export.

### SEC-AC-027 — Backup classification
Backup artifacts are treated as recovery/project copies, not review packages.

### SEC-AC-028 — Import preview
Importing a package shows a reviewable result before mutation.

### SEC-AC-029 — Import no overwrite
A conflicting import does not silently overwrite host data.

### SEC-AC-030 — Identity safety
Import distinguishes existing-object changes from newly copied objects.

### SEC-AC-031 — Project collision safety
Same-project identity collision provides preservation-oriented choices.

### SEC-AC-032 — Missing references
Missing external references are reported.

### SEC-AC-033 — Recovery safety
Crash recovery offers a user decision rather than silently replacing saved state.

### SEC-AC-034 — Deleted-state privacy
Recoverable deleted objects remain access-controlled.

### SEC-AC-035 — Activity privacy
Activity History is read-only and permission-scoped.

### SEC-AC-036 — No keystroke audit
Ordinary typing is not emitted as visible activity events.

### SEC-AC-037 — LAN scope
Participants see only the shared project scope allowed to their role.

### SEC-AC-038 — LAN private-note protection
Private Notes remain private during LAN sessions.

### SEC-AC-039 — LAN session lifetime
Ending the LAN session removes temporary shared context without cloud publication.

### SEC-AC-040 — LAN conflict safety
Same-object conflicts remain visible and no silent last-write-wins data loss occurs.

### SEC-AC-041 — Disconnect safety
Participant disconnect preserves acknowledged changes and local work where possible.

### SEC-AC-042 — AI collaboration boundary
AI during collaboration cannot access host-only, unauthorized, or another participant's private data.

### SEC-AC-043 — Offline parity
Offline operation does not weaken permission/privacy rules.

### SEC-AC-044 — Drive-loss behavior
External-drive loss does not create a silent divergent project.

### SEC-AC-045 — Save truthfulness
Save status accurately reflects local persistence.

### SEC-AC-046 — Security truthfulness
The product does not claim unsupported encryption, provider-retention, cloud-backup, or revocation guarantees.

### SEC-AC-047 — Historical integrity
Historical snapshots remain immutable representations of their captured state.

### SEC-AC-048 — Approval/permission separation
Explicit user approval does not bypass role/object permissions.

### SEC-AC-049 — Provider boundary
External provider processing is disclosed without being represented as OpenFrame-owned project storage.

### SEC-AC-050 — Ownership clarity
The application consistently communicates that the project is the filmmaker's work.

---

# 65. Security QA Scenarios

## Scenario 1 — Viewer + AI

1. Join project as Viewer.
2. Ask AI to rename a character.
3. AI resolves intent.
4. Permission check fails.
5. No Change Set can become applicable.
6. Project remains unchanged.

Expected: no mutation.

## Scenario 2 — Private Note + external AI

1. Owner creates a Private Note.
2. Another collaborator asks external AI about it.
3. AI retrieval excludes the Private Note.
4. The external request contains no protected private-note content.

Expected: denied/limited response without disclosure.

## Scenario 3 — Normal screenplay export

1. Create private scene note.
2. Export screenplay PDF.
3. Inspect output.

Expected: private note absent.

## Scenario 4 — Review exchange

1. Select screenplay review package.
2. Export package.
3. Inspect package contents through supported preview/import flow.

Expected: Private Notes absent.

## Scenario 5 — Full project package

1. Create project with supported private information.
2. Export Full Project Package.
3. Review package classification.

Expected: package is clearly identified as full project transfer/portability, not a review package.

## Scenario 6 — Stale AI proposal

1. Ask AI to prepare a multi-object Change Set.
2. Modify one affected object manually.
3. Attempt to apply previous AI proposal.

Expected: Stale/Conflict handling and revalidation; no blind apply.

## Scenario 7 — LAN Story-only scope

1. Host shares Story only.
2. Participant asks AI for private screenplay information.

Expected: no screenplay/private-note retrieval.

## Scenario 8 — LAN disconnect

1. Participant edits permitted content.
2. Network disconnects.
3. Host continues local work.
4. Participant retains local state where possible.

Expected: no silent data loss.

## Scenario 9 — Exchange conflict

1. Export review package from an old source version.
2. Change host project.
3. Import package.

Expected: stale/conflict comparison and review; no blind overwrite.

## Scenario 10 — Drive removal

1. Open project from external drive.
2. Remove/make drive unavailable.
3. Continue editing.

Expected: warning, no hidden replacement project, safe recovery path.

## Scenario 11 — Import failure

1. Import a supported but invalid/corrupt source.
2. Parsing fails.

Expected: project remains unchanged.

## Scenario 12 — Cross-project query

1. Open Project A.
2. Keep Project B available on same machine.
3. Ask a project-scoped AI question about A.

Expected: no Project B information appears.

---

# 66. Security Design Gaps That Must Remain Explicit

The current source documents give OpenFrame a strong product-level security/privacy model, but they intentionally do not fully specify every technical security mechanism.

The following should therefore remain visible engineering decisions rather than hidden assumptions:

## 66.1 LAN authentication mechanism

Defined outcome:

> Only admitted users with granted role and shared scope may participate.

Not defined:

- exact handshake;
- credentials;
- pairing code;
- certificate model;
- network protocol.

## 66.2 Encryption

Defined outcome:

> The product must not expose or claim unsupported protection.

Not defined:

- file-level encryption algorithm;
- transport encryption algorithm;
- key-management system.

## 66.3 OS-level file permissions

The product assumes ordinary local/external storage behavior and user-controlled files.

Exact OS permission integration is not defined.

## 66.4 External provider retention

The product requires disclosure of external processing.

Exact provider retention/deletion policy remains external unless explicitly integrated into provider configuration/documentation.

## 66.5 Telemetry

The current baseline does not establish a mandatory project-content telemetry system.

Therefore no product requirement should silently introduce one.

Any future telemetry that transmits project content requires an explicit separate requirement and privacy review.

---

# 67. Security Non-Negotiables

The entire specification can be reduced to the following contract:

```text
User-Owned Project
+
Local Primary State
+
Explicit Permissions
+
Private-Note Boundary
+
Scoped Collaboration
+
Safe Import / Export
+
Visible External AI Disclosure
+
AI Permission Inheritance
+
No Direct AI Write Authority
+
No Silent Overwrite
+
Recoverable Local State
+
Accurate Security Claims
```

If an implementation violates any of these, it is not aligned with the current OpenFrame product definition.

---

# 68. Final Cross-Document Consistency Matrix

| Security / Privacy Rule | PRD | FSD | UX/UI | Domain/Data | AI | Import/Export | Offline/Collab |
|---|---|---|---|---|---|---|---|
| Local ownership | Yes | Yes | Yes | Yes | Yes | Yes | Yes |
| No mandatory OpenFrame cloud | Yes | Yes | Yes | Yes | Yes | Yes | Yes |
| Private Notes | Yes | Yes | Yes | Yes | Yes | Yes | Yes |
| Permissions | Yes | Yes | Yes | Yes | Yes | Yes | Yes |
| Owner/Editor/Commenter/Viewer/Export-only | Yes | Yes | Yes | Role context | Inherited | Enforced | Enforced |
| External AI disclosure | Yes | Yes | Yes | External-processing state | Normative | Preserved | Preserved |
| AI permission inheritance | Yes | Yes | UX states | Authorization state | Normative | Cannot bypass | Cannot bypass |
| AI direct-write prohibition | Yes | Yes | Approval UI | Tool invocation/change set | Normative | Preserved | Preserved |
| Export snapshot | Yes | Yes | Yes | Snapshot | Cannot bypass | Normative | Supported offline |
| Review package privacy | Yes | Yes | Yes | Exchange Package | Scoped | Normative | Remote fallback |
| Full project package distinction | Yes | Yes | Yes | Package/snapshot | Scoped | Normative | Supported |
| Offline local work | Yes | Yes | Yes | Project state | Local AI | Import/export | Normative |
| LAN shared scope | Yes | Yes | Yes | Collaboration Session | Inherited | Package fallback | Normative |
| Visible conflict handling | Yes | Yes | Yes | Change Set/version | Stale protection | Import safety | Normative |
| Recovery/backup | Yes | Yes | Yes | Snapshot/deleted state | AI failure safe | Backup before risk | Normative |
| No silent overwrite | Yes | Yes | Yes | Change Set | AI stale/conflict | Normative | Normative |
| Project source-of-truth separation | Yes | Yes | Yes | Canonical authority | AI non-authority | Snapshot/export | Shared-project preservation |

---

# 69. Final Definition

OpenFrame Studio's security and privacy model is intentionally consistent with its local-first product identity.

It does not attempt to become a cloud-controlled production platform.

Instead:

```text
                 FILMMAKER / USER
                        │
                        ▼
              USER-OWNED PROJECT
                        │
        ┌───────────────┼────────────────┐
        ▼               ▼                ▼
   Local OpenFrame   Local AI       Collaboration
        │               │                │
        │               │         ┌──────┴──────┐
        │               │         ▼             ▼
        │               │       LAN         Exchange
        │               │         │             │
        └───────────────┴─────────┴─────────────┘
                        │
                        ▼
               Explicit Permission
                        │
                        ▼
                Scoped Data Access
                        │
             ┌──────────┴──────────┐
             ▼                     ▼
        Local mutation       External transmission
             │                     │
             ▼                     ▼
      Undo / Activity       Disclosure + consent
```

The central rule is:

> **OpenFrame should make it easy to work with other people and optional AI without making the filmmaker surrender control of the project's data.**

The product therefore protects ownership through local persistence, privacy through explicit scope and Private Note isolation, collaboration through roles and temporary sessions, portability through safe packages, and AI through permission inheritance, deterministic application tools, external-processing disclosure, and explicit mutation approval.

This security model is intentionally simple enough to fit OpenFrame's independent-filmmaker philosophy while being strict enough to prevent the major classes of accidental disclosure, unauthorized mutation, silent overwrite, and false ownership assumptions already identified across the aligned project specifications.

---

# Appendix A — Consistency Audit Performed for This Specification

The specification was written against the current seven-file baseline and was checked for alignment with the following source-level facts:

1. The PRD defines local project ownership, offline operation, exportability, Private Notes, external-AI disclosure, simple collaboration permissions, Exchange Packages, LAN collaboration, and activity history.
2. The FSD defines the canonical collaboration roles, Private Note behavior, offline save/recovery/backup behavior, exchange package safety, LAN scope, and detailed AI permission/disclosure/Change Set behavior.
3. The UX/UI specification defines private-note indicators, permission UI, export/import preview, external-AI disclosure states, offline/session indicators, and read-only Activity History.
4. The Domain/Data specification defines Application User, Project ownership, Private Note ownership, Project File references, Exchange Package, Import Session, Collaboration Session, AI Request/Result, Change Set, Snapshot, Deleted Item, and Activity Entry.
5. The AI specification defines AI permission inheritance, Private Note protection, local/external processing distinction, external disclosure, direct-write prohibition, Change Set approval, base-version validation, and activity integration.
6. The Import/Export specification defines the four portability concepts, private-note export boundaries, package scope, validation, identity mapping, stale/conflict handling, missing references, and project collision behavior.
7. The Offline/Collaboration specification defines Solo Local, Exchange Collaboration, LAN Collaboration, local ownership, private-note protection, role inheritance, shared-scope control, disconnect safety, visible conflicts, external-AI disclosure during collaboration, and Exchange fallback.

No mandatory cloud ownership or enterprise security subsystem has been added to the product scope.

---

# Appendix B — Terminology Lock

The following terms should remain stable across future specifications:

| Preferred term | Do not silently replace with |
|---|---|
| Application User | Enterprise account / HR user |
| Owner | Administrator, unless describing a separate technical concept |
| Editor | Contributor when referring to the canonical collaboration role |
| Commenter | Reviewer when referring to access permission |
| Viewer | Read-only collaborator only |
| Export-only | Download-only account system |
| Private Note | Shared note |
| Exchange Package | Live sync object |
| Full Project Package | Review package |
| Backup Package | Collaboration package |
| Collaboration Session | Permanent shared project |
| Snapshot | Live synchronized copy |
| Change Set | Hidden mutation script |
| Activity Entry | Full telemetry log |
| Local AI | Cloud AI |
| External AI | Local AI |
| Project owner | Platform owner |

---

# Appendix C — Requirement ID Summary

**Security requirements:** `SEC-001` through `SEC-060`

**Security acceptance criteria:** `SEC-AC-001` through `SEC-AC-050`

These IDs are local to this specification. They do not replace PRD or FSD requirement IDs.

The existing PRD/FSD IDs remain the product's established traceability anchors.