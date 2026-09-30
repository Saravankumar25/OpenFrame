# OpenFrame Studio — Offline & Collaboration Specification

**Document type:** Cross-cutting offline operation, local ownership, save/recovery, backup, external storage, exchange collaboration, LAN collaboration, permissions, presence, conflict resolution, reconnection, and AI interaction specification

**Product:** OpenFrame Studio

**Platform:** Windows + macOS desktop application

**Operating model:** Local-first, offline-capable, user-owned project files; optional local AI; optional external AI; optional local-network collaboration; no mandatory OpenFrame cloud

**Source baseline:**
- `OpenFrame_Studio_Mega_PRD_Aligned_Updated(1).md`
- `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated(1).md`
- `OpenFrame_Studio_UX_UI_Specification_Updated(1).md`
- `OpenFrame_Studio_Domain_Data_Specification_Updated(1).md`
- `OpenFrame_Studio_AI_Specification_Updated(1).md`

**Purpose:** Define how OpenFrame Studio behaves when the internet is unavailable, when local storage is used, when projects live on external drives, when collaborators exchange files remotely, when users collaborate on the same local network, when connectivity fails during a session, and when conflicts must be reconciled without losing user work.

**Status:** Cross-cutting offline and collaboration baseline. It specializes the behavior established by the five core specifications and does not introduce a mandatory cloud platform.

---

# 0. Specification Contract

## 0.1 Core idea

OpenFrame is a **local-first desktop application**.

The user's project remains their local working asset.

Internet connectivity is optional infrastructure, not a prerequisite for ownership of or ordinary access to the project.

The product therefore supports three collaboration modes:

```text
Mode 1 — Solo Local
Mode 2 — Exchange Packages
Mode 3 — Local-Network Collaboration
```

These are complementary rather than competing models.

## 0.2 Authority

The PRD owns product scope and priorities.

The FSD owns observable functional behavior.

The UX/UI specification owns presentation and interaction.

The Domain/Data specification owns logical identity, ownership, relationships, versioning, and state.

The AI specification owns AI-specific behavior across these layers.

This document specializes the offline/collaboration behavior and must not contradict those sources.

## 0.3 Non-negotiable rules

1. A user can open and work on their local project without internet.
2. Local save state remains the primary source of project persistence.
3. Internet loss must not silently create a divergent replacement project.
4. Core filmmaking workflows remain available offline.
5. Local AI may remain available offline when configured.
6. External AI is unavailable without network access to its provider.
7. No OpenFrame cloud account is required for ordinary local use.
8. Remote collaboration can use file-based Exchange Packages.
9. Local-network collaboration is optional.
10. Collaboration permissions remain active during LAN sessions.
11. Private notes remain private.
12. Disconnect must not silently destroy or overwrite work.
13. Same-object conflicts must be visible.
14. No silent last-write-wins data loss.
15. Collaboration session state is temporary and is not a permanent project permission.
16. Ending a LAN session does not publish the project to OpenFrame cloud.
17. Exchange Packages remain the safe fallback when users are not connected to the same network.
18. AI cannot bypass offline, permission, conflict, or approval rules.

---

# 1. Operating Modes

## 1.1 Solo Local

Default mode.

One person opens a local project and works without requiring any network.

Available operations include:

- project creation;
- project open/close;
- Idea Vault;
- Story Board;
- Screenplay;
- drafts;
- comments/review locally;
- breakdown;
- Production Catalog;
- locations;
- cast/crew;
- moodboards;
- storyboards;
- shot lists;
- schedule;
- call sheets;
- notes/tasks;
- reports;
- supported imports;
- supported exports;
- backup;
- recovery.

## 1.2 Exchange Collaboration

Used when collaborators are remote or do not share a private network.

Workflow:

```text
Export Exchange Package
      ↓
Normal file transfer
      ↓
Reviewer/Collaborator
      ↓
Import Package
      ↓
Review/Map/Compare
      ↓
Export Response Package
      ↓
Originator imports/reviews
```

The communication channel can be email, messaging, USB, file-sharing, or another ordinary file-transfer method.

OpenFrame does not own the transport.

## 1.3 Local-Network Collaboration

Used when users are on the same private network.

One OpenFrame installation becomes the host for a temporary project session.

Participants connect to the host.

The host selects the shared scope and permissions.

The project remains locally owned.

---

# 2. Offline State

## 2.1 Offline is a normal state

The user interface must not treat:

> Offline

as:

> Error.

A compact status indicator is sufficient.

The user should be able to continue working without repeatedly being told to reconnect.

## 2.2 Core offline workflows

The following remain available without internet:

| Workflow | Offline |
|---|---|
| Create project | Yes |
| Open project | Yes |
| Edit Idea Vault | Yes |
| Edit Story Board | Yes |
| Write screenplay | Yes |
| Create drafts | Yes |
| Review/comments locally | Yes |
| Breakdown | Yes |
| Production Catalog | Yes |
| Locations | Yes |
| Cast/Crew | Yes |
| Shot List | Yes |
| Storyboard | Yes |
| Scheduling | Yes |
| Call Sheets | Yes |
| Save | Yes |
| Recovery | Yes |
| Backup | Yes |
| Supported import | Yes |
| Supported export | Yes |
| Full Project Package | Yes |
| Exchange Package | Yes |
| External/cloud AI | No, unless provider is reachable |
| Configured local AI | Yes, subject to local model availability |
| Local-network collaboration | Requires local network |

## 2.3 What offline must not do

Offline mode must not:

- require sign-in to OpenFrame cloud;
- block project opening because synchronization is unavailable;
- create a hidden replacement copy of the project;
- claim that the project is cloud-backed;
- silently discard changes made during disconnection.

---

# 3. Local Project Ownership

## 3.1 Local project as saved authority

The local project is the primary saved state.

Status indicators such as:

- Saved;
- Saving;
- Save Error;

refer to local persistence.

They do not imply cloud synchronization.

## 3.2 External drives

A project may live on:

- internal disk;
- external SSD/HDD;
- another writable mounted volume supported by the desktop environment.

The application should continue to use the opened project identity while the volume is available.

## 3.3 Drive removal

If the project volume disappears:

- preserve current in-memory state where possible;
- inform the user;
- do not create a separate silent replacement project;
- allow recovery/reopen when the volume returns;
- offer safe recovery/export if supported.

The central rule is:

> **Do not manufacture a second truth because the original storage temporarily disappeared.**

---

# 4. Save and Autosave

## 4.1 Autosave

The application should persist normal edits automatically.

Users should not need to press Save repeatedly to avoid loss.

## 4.2 Manual Save

A conventional Save command remains available.

## 4.3 Save status

At minimum:

```text
Saved
Saving
Save Error
```

Where relevant, a contextual status can also indicate:

```text
Saved with pending package operation
```

## 4.4 Save error

A save error must:

- preserve current work in memory where possible;
- explain the problem;
- allow retry;
- offer recovery when appropriate;
- never falsely claim the project is safely saved.

---

# 5. Crash Recovery

## 5.1 Recovery points

The application may maintain Automatic History Points and other local recovery state as defined by the Domain/Data model.

## 5.2 Recovery on reopen

After abnormal shutdown, if recoverable state exists, show:

> We found a recent recovery state for this project.

Offer:

- Open Recovery;
- Keep Saved Version;
- Compare/Review when possible.

## 5.3 Recovery does not silently overwrite

Recovery must be a visible user decision.

No recovery copy should silently replace the confirmed project state.

---

# 6. Backups

## 6.1 User-controlled backup

The user can create a Backup Package at any time.

## 6.2 Backup destination

The destination should be user-selected.

The user may back up to another disk.

## 6.3 Backup content

A backup should include the supported internal project content needed to restore the project.

External references must be classified as:

- included/copied;
- external;
- missing;
- unsupported.

## 6.4 Backup naming

The product should include:

- project name;
- date/time;
- optional user label.

## 6.5 Backup before risky operation

A high-impact operation such as importing a full project package into an existing project may offer:

> Create Backup Before Import

This preserves the current state before the risk is taken.

---

# 7. Project Portability

## 7.1 Full Project Package

A Full Project Package is a supported way to move the working project to another machine.

It is different from:

- a screenplay PDF;
- an FDX file;
- a review exchange package;
- a backup-only artifact.

## 7.2 Portable project goal

A user should be able to:

```text
Machine A
Project
 ↓
Full Project Package
 ↓
Machine B
Open as project/copy
```

without needing OpenFrame cloud.

## 7.3 Identity collision

If the destination already contains the same project identity:

Preferred choices:

- Open as Copy;
- Replace Existing After Backup;
- Cancel.

Default should favor preservation.

---

# 8. Permissions

## 8.1 Roles

Current collaboration roles are:

- Owner;
- Editor;
- Commenter;
- Viewer;
- Export-only.

## 8.2 Owner

Can:

- manage project settings;
- manage permissions;
- manage collaboration session;
- edit permitted project content;
- perform owner-only destructive actions;
- end the collaboration session.

## 8.3 Editor

Can edit authorized project content but cannot take owner-only project control.

## 8.4 Commenter

Can:

- view permitted content;
- add/resolve comments where allowed.

Cannot edit core screenplay/story/production content.

## 8.5 Viewer

Read-only.

## 8.6 Export-only

Can create permitted snapshots/packages without editing project data.

Export-only users cannot enter a live editing session.

---

# 9. Permission Inheritance

Permissions apply consistently across:

- normal local project use;
- Exchange Packages;
- LAN collaboration;
- AI;
- import;
- export.

An AI assistant operates within the current user's effective permission boundary.

A participant cannot use AI to bypass their role.

A package cannot grant the receiver more permission than the package workflow allows.

---

# 10. Private Notes

Private notes remain visible only to the owner/authorized private context.

This remains true:

- offline;
- in Exchange Packages;
- in LAN collaboration;
- when AI is used;
- during export;
- during review.

Private notes do not become shared merely because the entire project is selected for collaboration.

A user can explicitly turn information into ordinary shared project content through the normal application workflow.

---

# 11. Remote Collaboration Through Exchange Packages

## 11.1 Why this exists

OpenFrame should not require a cloud platform merely to send a screenplay to another person.

## 11.2 Remote workflow

```text
Author
  ↓
Select workspace/scope
  ↓
Export Exchange Package
  ↓
Normal communication channel
  ↓
Collaborator
  ↓
Import Package
  ↓
Review
  ↓
Response Package
  ↓
Author imports
```

## 11.3 Remote transport

The transport layer can be:

- email;
- messaging;
- USB;
- user-selected file sharing;
- other ordinary transfer methods.

OpenFrame does not become responsible for the transport.

---

# 12. Local-Network Collaboration

## 12.1 Purpose

LAN collaboration provides simultaneous editing when participants share a private local network.

## 12.2 Host

The host:

- selects the project;
- selects shared scope;
- assigns participant roles;
- starts the session;
- controls the active session;
- can end the session.

## 12.3 Participant

A participant:

- joins using local session information;
- receives the assigned role;
- sees only the permitted shared scope;
- can leave the session;
- retains local work according to session behavior.

## 12.4 Shared scope

The host may select:

- entire project;
- Story Board;
- Screenplay;
- specific Production workspace.

The shared scope must not reveal excluded private areas.

---

# 13. Collaboration Session Lifecycle

Canonical lifecycle:

```text
Not Running
    ↓
Host Starting
    ↓
Running
    ↓
Participant Joined
    ↓
Participant Active
    ↓
Participant Disconnected
    ↓
Session Ended
```

A session may move among participant states as users join and leave.

## 13.1 Session state is temporary

The collaboration session is not a permanent project object representing ownership.

Ending it does not:

- delete the project;
- publish the project;
- upload the project to OpenFrame cloud;
- alter normal project permissions.

---

# 14. Presence

## 14.1 Purpose

Presence tells users:

> Who else is here?

and, where useful:

> Who is editing this?

## 14.2 Presence examples

- participant name;
- avatar/identity marker;
- current workspace;
- active Scene;
- active object.

## 14.3 Presence is informational

Presence does not become a permanent project field.

A disconnected collaborator may disappear from live presence while remaining a valid project user.

---

# 15. Soft Locks

## 15.1 Purpose

Soft locks reduce accidental simultaneous editing of high-conflict areas.

Possible lock targets:

- screenplay text region;
- scene;
- schedule block;
- complex catalog edit.

## 15.2 Behavior

The UI may display:

> Riya is editing this scene.

Other users may be asked to wait/request access, or the application may permit concurrent editing where safe.

## 15.3 No permanent lockout

A disconnected user must not leave an area permanently locked.

The session may release the lock when the participant is known to have disconnected.

An authorized host may also release a stale lock where appropriate.

## 15.4 Session-only

Soft-lock state exists in the active collaboration session.

It is not stored as a permanent project restriction.

---

# 16. Conflict Principles

The conflict model prioritizes preservation of work.

## 16.1 Different objects

Independent changes to different objects should coexist where safe.

Example:

```text
User A edits Scene Card 12
User B edits Location 4
```

No conflict.

## 16.2 Same object, different fields

If users edit independent fields of the same object:

```text
User A changes description
User B changes note
```

the system should preserve both changes when they are independently representable.

## 16.3 Same text region

If two users edit the same screenplay text region and the changes cannot be safely combined:

> Create a visible conflict.

Never silently choose a version.

## 16.4 Schedule conflicts

Schedule changes can conflict when:

- the same scene is moved by different users;
- shooting-day assignments diverge;
- day order changes;
- a day is deleted or changed concurrently.

These should be reviewed rather than silently losing one participant's work.

---

# 17. Conflict Resolution UI

The user should see enough context to understand the choice.

Example:

```text
Conflict in Scene 24

Your version
...

Incoming version
...

Current working copy
...

[Keep Mine]
[Use Incoming]
[Review Differences]
[Manually Combine]
```

The interface should identify the affected object and context.

## 17.1 No silent last-write-wins

A later-arriving edit must not automatically destroy an earlier conflicting edit simply because it arrived later.

---

# 18. Disconnect Behavior

## 18.1 Participant disconnect

If a participant disconnects:

- acknowledged shared changes remain in the session/project;
- user identity remains in the project permissions model;
- presence changes to disconnected;
- unacknowledged local work is retained locally where possible.

## 18.2 Host disconnect

If the host loses the network or terminates unexpectedly, connected participants must be informed that the shared session is unavailable.

They must not be told that the project has been deleted.

## 18.3 Local work preservation

Disconnected local work should remain available for safe recovery/export where supported.

---

# 19. Offline-After-Collaboration

A participant who continues working after losing the session must not automatically overwrite the host later.

Preferred path:

```text
Participant local work
      ↓
Recovery / Exchange Package
      ↓
Import Session
      ↓
Compare / Conflict Review
      ↓
Approved Change Set
```

This reuses the same safe exchange model as remote collaboration.

---

# 20. Reconnection

Reconnection should not be treated as:

> Automatically merge everything.

Instead:

1. identify session;
2. identify last acknowledged state;
3. identify local unacknowledged changes;
4. compare with host/current state;
5. surface conflicts where present;
6. resume safe shared work or route divergent local work through review.

No silent data loss.

---

# 21. Session Scope and Data Visibility

If the host shares Story only:

- Story Board content is available;
- unrelated private screenplay/production areas remain unavailable;
- AI can only retrieve data available within the participant's permitted scope.

If the host shares Screenplay:

- screenplay access follows participant role;
- private notes remain private.

If the host shares Production:

- only permitted production content is exposed.

---

# 22. Collaboration and Source of Truth

LAN collaboration does not create a second source of truth.

The shared project remains the same logical project.

The collaboration session is the mechanism by which permitted participants work on the currently shared local project context.

Exchange Packages remain snapshots.

Historical documents remain snapshots.

Call Sheet rules remain intact.

A schedule edit remains a schedule edit.

A screenplay edit remains a screenplay edit.

Collaboration must never collapse these distinct sources.

---

# 23. Collaboration and Versioning

## 23.1 Named drafts

Named screenplay drafts remain persistent project versions.

## 23.2 Automatic history

Automatic History Points remain recovery history.

## 23.3 Collaboration edits

Collaboration edits become ordinary project changes.

They do not create a special “collaboration screenplay” as a second canonical script.

## 23.4 Conflict resolution

A resolved conflict should produce a clear resulting project state and history/activity entry where supported.

---

# 24. Exchange Package Fallback

When users are not on the same network, OpenFrame uses Exchange Packages.

When a LAN session becomes unavailable, users should be able to fall back to:

```text
Export
→ Transfer
→ Import
→ Review
```

This means collaboration remains useful without requiring the same network to stay available.

---

# 25. Offline and AI

## 25.1 Local AI

A configured local model may continue operating offline.

It can use:

- accessible local project data;
- product knowledge available locally;
- deterministic project query tools;
- local application tools.

## 25.2 External AI

External provider calls require the appropriate network connection.

When unavailable:

- AI reports that the external provider cannot be reached;
- core project workflows continue;
- local AI may remain available if configured.

## 25.3 No AI dependency

OpenFrame must remain fully usable for its core workflows when AI is unavailable.

---

# 26. AI During Collaboration

AI operates under the current participant's permissions and shared scope.

It must not use:

- hidden host-only data;
- another participant's private notes;
- excluded workspaces;
- unauthorized objects.

## 26.1 AI reads

Read-only AI queries can execute directly within the user's permitted scope.

## 26.2 AI mutations

AI mutations still require the normal OpenFrame AI approval workflow:

```text
Request
→ Context
→ Permission
→ Proposal
→ Preview
→ User acceptance
→ Normal project action
```

Collaboration does not weaken this rule.

## 26.3 Concurrent AI changes

If a proposed AI Change Set was created against an earlier collaboration state, it must be revalidated before applying.

---

# 27. External AI and Collaboration Privacy

If external AI is used during a collaboration session, the user must receive the normal external-processing disclosure.

The disclosure should reflect the content being sent.

A participant's use of external AI does not grant the provider access to all project data.

Only the authorized/requested context should be transmitted.

---

# 28. Offline and Import/Export

Offline operation includes supported:

- screenplay imports;
- screenplay exports;
- exchange package exports;
- exchange package imports;
- full project package export/import;
- backup creation.

AI is not required for these workflows.

If AI-assisted mapping is unavailable offline, deterministic/manual mapping remains available.

---

# 29. Activity and Audit

Meaningful collaboration events may be recorded as Activity Entries.

Examples:

- collaboration session started;
- participant joined;
- participant left;
- project data changed;
- conflict resolved;
- session ended;
- exchange package imported;
- exchange response applied.

Presence itself need not create noisy permanent activity records.

The activity log is not a full network packet log.

---

# 30. Failure Safety

## 30.1 Network failure

Network failure cannot:

- delete the project;
- overwrite the local project with an empty state;
- silently publish anything;
- convert the project to cloud dependency.

## 30.2 Storage failure

Storage failure must not be misreported as network failure.

If the project cannot be persisted, the application must communicate the storage problem and provide the recovery path supported by the local project model.

## 30.3 Collaboration start failure

If LAN collaboration cannot start:

> Solo local work remains available.

## 30.4 Collaboration join failure

If a participant cannot join:

- no local project mutation occurs;
- explain the failure;
- allow local work.

## 30.5 Conflict processing failure

If a conflict cannot be resolved automatically:

- preserve both sides;
- keep the conflict visible;
- do not select a winner silently.

---

# 31. Local Network Session Safety

The session must clearly identify:

- host;
- shared project;
- shared scope;
- participant;
- role;
- session state.

A participant should not mistake a session for permanent access.

Ending the session removes the temporary shared-session context but does not delete the project or change normal project permissions unless an authorized user explicitly changes permissions.

---

# 32. No Cloud Dependency

The following do not require OpenFrame cloud:

- ordinary local use;
- offline editing;
- local saving;
- local backups;
- local project packages;
- review Exchange Packages;
- LAN collaboration.

External services may be used optionally by the user, but OpenFrame's core ownership model remains local.

---

# 33. Collaboration UX Contract

## Host setup

```text
Start Collaboration Session

Project
[Current Project]

Shared Scope
[Entire Project]
[Story]
[Screenplay]
[Production]

Participants / Roles

[Start]
```

## Participant join

```text
Join Local Session

Session
Role
Shared Scope

[Join]
```

## Active session

The shell displays a compact indicator such as:

> Working in local shared session

## Presence

Near an active object:

> Riya — editing

## Conflict

```text
Conflict Detected

Your version
Incoming version
Context

[Keep Mine]
[Use Incoming]
[Review Differences]
```

## Offline

The shell shows:

> Offline

without blocking ordinary work.

---

# 34. Collaboration with Call Sheets and Schedule

The normal source-of-truth rules remain.

If one participant changes the schedule:

- the schedule changes;
- existing issued call sheets do not silently rewrite.

If a call sheet is later refreshed:

- the user reviews the change;
- the schedule is not changed by editing the Call Sheet.

Collaboration does not permit reverse synchronization that the normal product model forbids.

---

# 35. Collaboration with Production Source

If the Screenplay changes during collaboration:

- Production Source may become stale;
- breakdown/schedule/call-sheet dependencies may be flagged;
- production data does not silently rewrite itself.

This is the same behavior as solo use.

---

# 36. Collaboration with Locked Scripts

A collaboration participant cannot use LAN access to bypass screenplay lock.

If a locked draft requires revision:

- the normal post-lock revision workflow applies;
- permissions apply;
- AI cannot bypass it.

---

# 37. Exchange Packages and Permissions

A package does not automatically grant project-wide access.

The package itself defines the selected content.

The receiver can work only within the package workflow.

Import into a host project remains subject to the host user's permissions.

---

# 38. Data Model Dependencies

The Domain/Data specification already provides the core objects:

- Application User;
- Exchange Package;
- Import Session;
- Collaboration Session;
- Snapshot;
- Change Set;
- Project;
- Project File;
- Deleted Item;
- Activity Entry.

These remain canonical.

## 38.1 Collaboration Session

The existing logical session state is:

```text
Not Running
Host Starting
Running
Participant Joined
Participant Active
Participant Disconnected
Session Ended
```

## 38.2 Temporary session state

Presence, soft locks, connection state, and other live session signals are session-scoped.

They must not become permanent project truth unless the user intentionally performs a project operation.

---

# 39. Detailed Collaboration State Model

## Host

```text
Idle
 ↓
Starting
 ↓
Running
 ↓
Participant Join/Leave
 ↓
Ending
 ↓
Ended
```

## Participant

```text
Disconnected
 ↓
Joining
 ↓
Connected
 ↓
Active
 ↓
Disconnected
 ↘
  Rejoining
```

## Conflict

```text
No Conflict
 ↓
Conflict Detected
 ↓
Review
 ↓
Resolved
```

A conflict can return to Review if a new inconsistent state arrives before resolution.

---

# 40. Offline/Reconnection Matrix

| Situation | Required behavior |
|---|---|
| App opened with no internet | Open project normally |
| Internet lost while editing solo | Continue locally |
| Internet lost during external AI request | AI request fails safely; project unchanged |
| Internet lost during document export | Local export may continue; otherwise fail without project mutation |
| LAN lost during collaboration | Shared session unavailable; local work preserved |
| Participant disconnected | Presence updates; local state preserved |
| Host session ends | Project remains local |
| Participant reconnects | Revalidate current shared state |
| Participant changed same object while disconnected | Review/reconcile, no silent overwrite |
| External drive removed | Warn; do not create silent divergent project |
| Storage save fails | Show Save Error; preserve in-memory state where possible |
| Crash with recovery state | Offer recovery choice |
| No recovery state | Open last confirmed saved state |

---

# 41. Offline/Collaboration Acceptance Criteria

## OC-001

Project opens without internet.

## OC-002

Core editing continues offline.

## OC-003

Production workflows continue offline.

## OC-004

Supported exports work offline.

## OC-005

Supported local imports work offline.

## OC-006

Backup can be created offline.

## OC-007

Local project remains the primary persistence source.

## OC-008

Offline state never requires cloud synchronization to open the project.

## OC-009

External drive projects remain valid while the drive is available.

## OC-010

Drive loss does not create a silent divergent project.

## OC-011

Crash recovery can offer a recoverable state.

## OC-012

User can choose recovery versus confirmed saved state.

## OC-013

Collaboration roles are Owner, Editor, Commenter, Viewer, Export-only.

## OC-014

Export-only user cannot enter live editing.

## OC-015

Host can choose collaboration scope.

## OC-016

Participants see only permitted shared scope.

## OC-017

Private notes remain private during collaboration.

## OC-018

LAN session has visible host/session state.

## OC-019

Presence is visible where useful.

## OC-020

Soft locks can protect high-conflict editing areas.

## OC-021

Soft locks cannot permanently lock out a disconnected participant.

## OC-022

Different-object edits coexist where safe.

## OC-023

Independent same-object field edits can coexist where safe.

## OC-024

Same-text-region conflicts become visible.

## OC-025

Conflict UI provides enough context for user choice.

## OC-026

No silent last-write-wins data loss occurs.

## OC-027

Participant disconnect preserves acknowledged changes.

## OC-028

Unacknowledged local work is retained where possible.

## OC-029

Participant offline changes are reconciled through a safe review path.

## OC-030

Reconnection does not blindly overwrite the host.

## OC-031

Ending a LAN session does not upload/publish the project to OpenFrame cloud.

## OC-032

Remote collaboration can fall back to Exchange Packages.

## OC-033

Exchange Packages can be used without OpenFrame cloud.

## OC-034

AI respects collaboration permissions.

## OC-035

AI does not expose host-only or private participant data.

## OC-036

AI mutations still require explicit user approval.

## OC-037

External AI use is disclosed before project data is transmitted externally.

## OC-038

Core product behavior remains functional when AI is disabled.

## OC-039

Collaboration session state is temporary and does not become permanent project permission.

## OC-040

Session start failure does not block solo work.

## OC-041

Session join failure does not mutate local project content.

## OC-042

Conflict resolution creates a clear resulting project state.

## OC-043

Meaningful collaboration events can be represented through Activity History.

## OC-044

Call Sheet changes do not reverse-sync into the Schedule.

## OC-045

Schedule changes do not silently rewrite issued Call Sheets.

## OC-046

Production Source staleness remains visible after collaborative screenplay changes.

## OC-047

Locked screenplay protections remain active during LAN collaboration.

---

# 42. Traceability to Current Requirements

## PRD

| PRD ID | Relevant contract |
|---|---|
| PRD-COL-001 | Permissions |
| PRD-COL-002 | Review Packages |
| PRD-COL-003 | Page-Specific Exchange Packages |
| PRD-COL-004 | Better Exchange Package Controls |
| PRD-COL-005 | Basic Local-Network Collaboration |
| PRD-COL-006 | Advanced Collaboration Conflict Handling |
| PRD-OFF-001 | Offline Operation, Saving, Portability, External Storage and Backup |
| PRD-AI-001 | AI optionality and controlled AI operation |
| PRD-AI-002 | Advanced AI project assistance |

## FSD

| FSD ID | Relevant contract |
|---|---|
| FSD-COL-001..005 | Roles and permissions |
| FSD-COL-006..011 | Exchange package types |
| FSD-COL-012..016 | Preview, stale, conflict, ambiguous and unmapped imports |
| FSD-COL-017..022 | LAN host/join/presence/conflict/disconnect/end |
| FSD-OFF-001..010 | Offline, save, recovery, backup, portability, external drive |
| FSD-OFF-010 | Drive loss must not silently create a divergent project |
| FSD-AI-001..012 | AI optionality, scope, Q&A, suggestions, mutation preview/approval, disclosure and failure safety |
| FSD-AI-012 | AI failure safety and unchanged project state on failure |

---

# 43. Alignment with the Five Core Specifications

## PRD alignment

Preserves:

- local-first ownership;
- no mandatory cloud;
- three collaboration modes;
- package-based remote collaboration;
- LAN collaboration;
- simple roles;
- offline core workflow.

## FSD alignment

Preserves:

- offline open/edit/production/export;
- local save authority;
- backup/recovery;
- project portability;
- external-drive behavior;
- exchange validation;
- stale/conflict handling;
- LAN session states;
- soft locks;
- no blind overwrite.

## UX/UI alignment

Preserves:

- compact status strip;
- Offline state;
- collaboration indicator;
- import preview;
- exchange comparison;
- host/participant setup;
- presence;
- conflict panel;
- recovery dialog.

## Domain/Data alignment

Preserves:

- Project as local working object;
- Snapshot;
- Exchange Package;
- Import Session;
- Collaboration Session;
- Application User;
- Change Set;
- Deleted Item;
- stable object identities.

## AI alignment

Preserves:

- AI as optional;
- scope-aware access;
- permission inheritance;
- deterministic data authority;
- no silent mutation;
- approval-gated Change Sets;
- local AI offline;
- external disclosure;
- collaboration-safe AI.

---

# 44. Important Boundary: Collaboration Is Not a New Source of Truth

OpenFrame has one logical project.

```text
Local Project
     ↑
Collaboration Session
     ↑
Participants
```

The session is not a second database.

An Exchange Package is not a live branch of the project.

A review response is not automatically canonical.

A participant's disconnected local work is not automatically canonical until safely reconciled.

This keeps the project coherent.

---

# 45. Important Boundary: Offline Is Not a Separate Product Mode

Offline is a state of normal OpenFrame operation, not a reduced “offline edition.”

The user should not feel as though they opened a different application.

The same project can move between:

```text
Offline
Online
LAN Collaboration
Exchange Collaboration
```

without changing its logical identity.

---

# 46. Final Offline & Collaboration Definition

OpenFrame Studio must let filmmakers own and work on their projects locally, collaborate remotely through portable exchange packages, and collaborate live on a private network without requiring OpenFrame cloud infrastructure.

The core safety model is:

```text
Local ownership
+
Explicit permissions
+
Temporary collaboration session
+
Portable exchange fallback
+
Visible conflicts
+
Recoverable local state
+
No silent overwrite
+
AI obeying the same boundaries
```

The goal is not to recreate a cloud collaboration service.

The goal is to let small filmmaking teams work together safely while the project remains portable, locally owned, understandable, and recoverable.
