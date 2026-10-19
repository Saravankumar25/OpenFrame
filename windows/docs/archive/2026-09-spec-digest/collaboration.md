# Digest — OpenFrame Studio Offline & Collaboration Specification

Source: `C:\dev\OpenFrame\OpenFrame_Studio_Offline_Collaboration_Specification.md` (1686 lines, ~5.1k words, read in full).
Cross-reference: `OpenFrame_Studio_Import_Export_Specification.md` (cited as **IE §n**). Package merge mechanics live there.
Section refs are to the Offline & Collaboration spec (**§n**) unless marked IE.

**How to read this digest**
- Everything here comes from the spec. Quoted text and `> blocks` are copied exactly.
- **[NOT SPECIFIED]** means the spec is silent on the point. Those items are gaps, not requirements. The spec is a behavioral contract. It does **not** define network protocol, discovery mechanism, ports, auth/pairing, lock timeouts, CRDT/OT algorithms or message wire formats. Section 30 of this digest lists these gaps.

---

## 0. Core idea and authority (§0)

- OpenFrame is a **local-first desktop application**. "The user's project remains their local working asset." Internet "is optional infrastructure, not a prerequisite for ownership of or ordinary access to the project."
- There are three complementary collaboration modes:
  ```text
  Mode 1 — Solo Local
  Mode 2 — Exchange Packages
  Mode 3 — Local-Network Collaboration
  ```
- Authority: PRD owns scope and priorities. FSD owns observable behavior. UX/UI owns presentation. Domain/Data owns identity, ownership, relationships, versioning and state. The AI spec owns AI behavior. This document "must not contradict those sources."

## 1. Non-negotiable rules (§0.3), verbatim

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

## 2. The three modes (§1)

| Mode | When | Mechanics | Key rules |
|---|---|---|---|
| **1. Solo Local** (default) | One person, no network needed | Local project open/edit | All core ops available: project create/open/close, Idea Vault, Story Board, Screenplay, drafts, comments/review locally, breakdown, Production Catalog, locations, cast/crew, moodboards, storyboards, shot lists, schedule, call sheets, notes/tasks, reports, supported imports, supported exports, backup, recovery |
| **2. Exchange Collaboration** | Collaborators are remote or share no private network | `Export Exchange Package → Normal file transfer → Reviewer/Collaborator → Import Package → Review/Map/Compare → Export Response Package → Originator imports/reviews` | The transport can be email, messaging, USB, file-sharing or other ordinary methods. "OpenFrame does not own the transport." |
| **3. Local-Network Collaboration** | Users share a private network | "One OpenFrame installation becomes the host for a temporary project session. Participants connect to the host. The host selects the shared scope and permissions." | "The project remains locally owned." |

"Offline is a state of normal OpenFrame operation, not a reduced 'offline edition.'" A project can move between `Offline / Online / LAN Collaboration / Exchange Collaboration` without changing its logical identity (§45).

---

## 3. Offline rules (§2)

### 3.1 Offline is normal (§2.1)
- The UI must not treat "Offline" as "Error." "A compact status indicator is sufficient."
- The user continues working "without repeatedly being told to reconnect."
- Shell label (§33): **"Offline"**, shown "without blocking ordinary work."

### 3.2 Offline availability table (§2.2)
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

### 3.3 Offline must NOT (§2.3)
- require sign-in to OpenFrame cloud;
- block project opening because synchronization is unavailable;
- create a hidden replacement copy of the project;
- claim that the project is cloud-backed;
- silently discard changes made during disconnection.

## 4. Local ownership and storage (§3)

- The local project is the primary saved state. The status labels **Saved**, **Saving** and **Save Error** refer to **local** persistence. "They do not imply cloud synchronization."
- A project may live on an internal disk, an external SSD/HDD, or another writable mounted volume. The app keeps using the opened project identity while the volume is available (OC-009).
- **Drive removal (§3.3):** preserve in-memory state where possible; inform the user; do not create a silent replacement project; allow recovery/reopen when the volume returns; offer safe recovery/export if supported.
  > **Do not manufacture a second truth because the original storage temporarily disappeared.**
- **Storage vs network (§30.2):** "Storage failure must not be misreported as network failure." The app communicates the storage problem and the local recovery path.

## 5. Save and autosave (§4)

- Normal edits autosave. "Users should not need to press Save repeatedly to avoid loss." A manual **Save** command remains available.
- Minimum status set:
  ```text
  Saved
  Saving
  Save Error
  ```
- Contextual status: **"Saved with pending package operation"**.
- A save error must: preserve current work in memory where possible; explain the problem; allow retry; offer recovery when appropriate; "never falsely claim the project is safely saved."

## 6. Crash recovery (§5)

- The app may keep Automatic History Points and other local recovery state (per Domain/Data).
- After abnormal shutdown, if recoverable state exists:
  > We found a recent recovery state for this project.
- Options: **Open Recovery**; **Keep Saved Version**; **Compare/Review** when possible.
- Recovery "must be a visible user decision. No recovery copy should silently replace the confirmed project state."
- With no recovery state: "Open last confirmed saved state" (§40).

## 7. Backups (§6)

- The user can create a Backup Package at any time, to a **user-selected** destination (another disk allowed).
- Content: the supported internal project content needed to restore. External references are classified as **included/copied; external; missing; unsupported**. (IE §8.3 wording: included; external; missing; skipped/unsupported.)
- Naming includes: **project name; date/time; optional user label**. The exact format is [NOT SPECIFIED].
- High-impact operations such as importing a full project package into an existing project may offer:
  > Create Backup Before Import

## 8. Portability (§7)

- The Full Project Package is distinct from a screenplay PDF, an FDX, a review exchange package, and a backup-only artifact.
- Flow: `Machine A Project → Full Project Package → Machine B Open as project/copy`, with no cloud.
- **Identity collision:** **Open as Copy**; **Replace Existing After Backup**; **Cancel**. "Default should favor preservation."

---

## 9. Roles and permissions (§8–§10)

### 9.1 Roles (§8.1, OC-013)
**Owner, Editor, Commenter, Viewer, Export-only.**

### 9.2 Capability matrix, derived from §8.2–§8.6

| Capability | Owner | Editor | Commenter | Viewer | Export-only |
|---|---|---|---|---|---|
| Manage project settings | Yes | No ("cannot take owner-only project control") | No | No | No |
| Manage permissions | Yes | No | No | No | No |
| Manage collaboration session / end session | Yes | No | No | No | No |
| Edit permitted project content | Yes | Yes ("authorized project content") | **No** ("Cannot edit core screenplay/story/production content") | No | No ("without editing project data") |
| Owner-only destructive actions | Yes | No | No | No | No |
| View permitted content | Yes | Yes | Yes | Yes (read-only) | [implied, NOT SPECIFIED] |
| Add/resolve comments | [implied] | [NOT SPECIFIED] | Yes, "where allowed" | No (read-only) | No |
| Create permitted snapshots/packages | [NOT SPECIFIED] | [NOT SPECIFIED] | [NOT SPECIFIED] | [NOT SPECIFIED] | Yes |
| Enter live (LAN) editing session | Yes | Yes | [NOT SPECIFIED] | [NOT SPECIFIED] | **No** ("Export-only users cannot enter a live editing session", OC-014) |

Only the cells quoted or described in §8 are sourced. Cells marked [implied] or [NOT SPECIFIED] need FSD-COL-001..005.

### 9.3 Permission inheritance (§9)
- Permissions apply consistently across local use, Exchange Packages, LAN collaboration, AI, import and export.
- AI "operates within the current user's effective permission boundary". "A participant cannot use AI to bypass their role."
- "A package cannot grant the receiver more permission than the package workflow allows."
- §37: "A package does not automatically grant project-wide access." "The receiver can work only within the package workflow." "Import into a host project remains subject to the host user's permissions."

### 9.4 Private notes (§10)
- Visible only to the owner/authorized private context. This holds offline, in Exchange Packages, in LAN collaboration, with AI, during export and during review.
- "Private notes do not become shared merely because the entire project is selected for collaboration."
- A user can explicitly turn information into ordinary shared content through the normal workflow.

---

## 10. Mode 2: Exchange Packages (§11, §24, §37, plus IE §9–§20)

### 10.1 Workflow (§11.2)
`Author → Select workspace/scope → Export Exchange Package → Normal communication channel → Collaborator → Import Package → Review → Response Package → Author imports`

### 10.2 Package types (defined in IE §9.2 / §11; FSD-COL-006..011)
| Family | Conceptual ext. | Contents (IE §11) | Import rule |
|---|---|---|---|
| Story Board | `.ofstory` | Acts, Sequences, Beats, Scene Cards, selected comments/attachments | Results: new outline copy / selected object changes / comments / review record |
| Screenplay Review | `.ofscriptreview` | source draft identity, screenplay snapshot, selected scene scope, comments/annotations, optional attachments | Default: attach comments to the matching draft; preserve the current screenplay; surface text changes for explicit review |
| Breakdown | `.ofbreakdown` | source scene identities, breakdown elements, catalog references, selected notes/comments | May prepare production-data updates after validation and review |
| Shot List | `.ofshots` | scene refs, shots, shot-planning fields, optional Storyboard links, attachments/comments | General pipeline |
| Schedule | `.ofschedule` | shooting days, scene assignments, markers, day notes, planning metadata | Must not silently delete or reorder local schedule data |
| Call Sheet review | `.ofcallreview` | call-sheet snapshot, source Shooting Day context, comments/review notes | Feeds the Call Sheet context; does not silently modify the main schedule |

The extensions "remain conceptual" (IE §9.2).

### 10.3 Merge rules for incoming packages (IE §12–§20, summarized)
- **Import Session** holds: source, type, compatibility, identity, version, mappings, conflicts, warnings, exclusions, proposed ops, selections, result.
- **Pipeline:** validate structure → format version → metadata → identity → resolve mappings → detect stale → conflicts → ambiguous → unmapped → build Change Set → preview → user decision → apply selected. "no path from 'open package' directly to 'overwrite current project.'"
- **Identity:** same carried identity → *propose update*. Intentional copy → *new identity*. No identity → contextual rules plus surfaced ambiguity. Scene numbers are never identity. Deleted host object → do not recreate; message "Incoming package refers to an object that no longer exists in the host project."
- **Stale:** "This package was created from an older project state." Options: inspect; import safe comments only; separate review record; compare versions; select safe changes; cancel.
- **Ambiguous:** never choose silently. Route to review.
- **Unmapped:** retain in "Unmatched Review Notes / Review Queue". Never discard or invent an attachment point.
- **Preview choices:** Import All Safe; Comments Only; Selected Items; Copy as New; Review Ambiguous; Cancel.
- **Result states:** Applied; Partially Applied; Pending Review; Rejected; Failed. Never show "Import complete" for a subset.
- The **approved Change Set is the change**. Pre-apply failure means zero mutation.

### 10.4 Fallback (§24)
When not on the same network, or when a LAN session becomes unavailable: `Export → Transfer → Import → Review`.

---

## 11. Mode 3: LAN collaboration (§12)

### 11.1 Purpose
"LAN collaboration provides simultaneous editing when participants share a private local network."

### 11.2 Host responsibilities (§12.2)
- selects the project;
- selects shared scope;
- assigns participant roles;
- starts the session;
- controls the active session;
- can end the session.

### 11.3 Participant (§12.3)
- "joins using local session information";
- receives the assigned role;
- sees only the permitted shared scope;
- can leave the session;
- "retains local work according to session behavior."

### 11.4 Shared scope options (§12.4)
**Entire project**; **Story Board**; **Screenplay**; **specific Production workspace**. "The shared scope must not reveal excluded private areas."

UI labels in the host dialog (§33): `[Entire Project] [Story] [Screenplay] [Production]`.

### 11.5 Scope visibility (§21)
- **Story only:** Story Board content is available. Unrelated private screenplay/production areas stay unavailable. AI retrieves only permitted-scope data.
- **Screenplay:** access follows participant role. Private notes stay private.
- **Production:** "only permitted production content is exposed."

### 11.6 Session safety identification (§31)
The session must clearly identify: **host; shared project; shared scope; participant; role; session state**.
- "A participant should not mistake a session for permanent access."
- Ending the session removes the temporary shared-session context. It does not delete the project or change normal permissions unless an authorized user explicitly changes them.

---

## 12. Session state models

### 12.1 Canonical Collaboration Session states (§13, §38.1, Domain/Data)
```text
Not Running
Host Starting
Running
Participant Joined
Participant Active
Participant Disconnected
Session Ended
```
"A session may move among participant states as users join and leave."

### 12.2 Detailed host state machine (§39)
`Idle → Starting → Running → Participant Join/Leave → Ending → Ended`

### 12.3 Detailed participant state machine (§39)
`Disconnected → Joining → Connected → Active → Disconnected`, and from Disconnected `↘ Rejoining`

### 12.4 Conflict state machine (§39)
`No Conflict → Conflict Detected → Review → Resolved`
"A conflict can return to Review if a new inconsistent state arrives before resolution."

Suggested mapping from canonical to detailed states (derived; the spec does not map them): Not Running ≈ Idle; Host Starting ≈ Starting; Running ≈ Running; Participant Joined/Active/Disconnected ≈ participant Connected/Active/Disconnected; Session Ended ≈ Ended (via Ending).

### 12.5 Session state is temporary (§13.1, §38.2)
- "The collaboration session is not a permanent project object representing ownership."
- Ending it does not delete the project, publish it, upload it to OpenFrame cloud, or alter normal project permissions.
- "Presence, soft locks, connection state, and other live session signals are session-scoped. They must not become permanent project truth unless the user intentionally performs a project operation."

---

## 13. Host setup, discovery and join flow (§33, §12.3)

### 13.1 Host setup dialog (§33), exact labels
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

### 13.2 Participant join dialog (§33), exact labels
```text
Join Local Session

Session
Role
Shared Scope

[Join]
```

### 13.3 Discovery
- The spec says only that a participant "joins using local session information" (§12.3).
- The discovery mechanism (mDNS/Bonjour, broadcast, manual IP, QR/code), authentication/pairing, invitation flow and how the host pre-assigns roles to not-yet-known participants are all [NOT SPECIFIED].

### 13.4 Active-session indicator (§33)
> Working in local shared session

It is a compact indicator in the shell.

### 13.5 Start and join failure (§30.3–§30.4)
- Start failure: "Solo local work remains available." (OC-040)
- Join failure: "no local project mutation occurs"; "explain the failure"; "allow local work." (OC-041)

---

## 14. Presence (§14)

- Purpose: "Who else is here?" and, where useful, "Who is editing this?"
- Presence data examples: participant name; avatar/identity marker; current workspace; active Scene; active object.
- Label near an active object (§33):
  > Riya — editing
- "Presence does not become a permanent project field." A disconnected collaborator may disappear from live presence while remaining a valid project user.
- Presence "need not create noisy permanent activity records" (§29).

## 15. Soft locks (§15)

- **Purpose:** "reduce accidental simultaneous editing of high-conflict areas."
- **Lock targets and granularity (the spec's list):**
  - screenplay text region;
  - scene;
  - schedule block;
  - complex catalog edit.
- **Message:**
  > Riya is editing this scene.
- **Behavior:** "Other users may be asked to wait/request access, or the application may permit concurrent editing where safe." Locks are *soft*, so blocking is optional.
- **No permanent lockout (§15.3):**
  - "A disconnected user must not leave an area permanently locked."
  - "The session may release the lock when the participant is known to have disconnected."
  - "An authorized host may also release a stale lock where appropriate."
- **Session-only (§15.4):** "not stored as a permanent project restriction."
- **Timeout:** [NOT SPECIFIED]. The spec gives no numeric lease/timeout. Release triggers are only (a) known disconnect and (b) authorized host manual release.
- The "request access" UI flow is [NOT SPECIFIED].

## 16. Versions, acknowledged state and stale-base detection

- **Collaboration edits are ordinary project changes (§23.3).** They "do not create a special 'collaboration screenplay' as a second canonical script."
- **Named drafts** remain persistent project versions. **Automatic History Points** remain recovery history (§23.1–§23.2).
- **Resolved conflicts** "should produce a clear resulting project state and history/activity entry where supported" (§23.4, OC-042).
- **Acknowledged vs unacknowledged:** shared changes the session has acknowledged persist in the session/project. Unacknowledged local work "is retained locally where possible" (§18.1). The acknowledgement protocol is [NOT SPECIFIED].
- **Stale AI proposals (§26.3):** "If a proposed AI Change Set was created against an earlier collaboration state, it must be revalidated before applying."
- **Stale packages:** IE §15 (see §10.3 above).
- **Production Source staleness (§35):** screenplay changes during collaboration may make the Production Source stale. Breakdown/schedule/call-sheet dependencies may be flagged. "production data does not silently rewrite itself" (OC-046).
- Version vectors, sequence numbers and the base-revision representation are [NOT SPECIFIED].

---

## 17. Conflict types and rules (§16, §30.5)

The principle: "The conflict model prioritizes preservation of work."

| Type | Example (verbatim) | Rule |
|---|---|---|
| **Different objects** (§16.1) | "User A edits Scene Card 12 / User B edits Location 4" | "No conflict." Changes coexist where safe (OC-022) |
| **Same object, different fields** (§16.2) | "User A changes description / User B changes note" | "preserve both changes when they are independently representable" (OC-023) |
| **Same text region** (§16.3) | Two users edit the same screenplay text region and the changes cannot be safely combined | "> Create a visible conflict." "Never silently choose a version." (OC-024) |
| **Schedule** (§16.4) | Same scene moved by different users; shooting-day assignments diverge; day order changes; a day is deleted or changed concurrently | "should be reviewed rather than silently losing one participant's work" |
| **Offline divergence** (§19, §40) | "Participant changed same object while disconnected" | "Review/reconcile, no silent overwrite" |

- **No silent last-write-wins (§17.1):** "A later-arriving edit must not automatically destroy an earlier conflicting edit simply because it arrived later." (OC-026)
- **Conflict processing failure (§30.5):** preserve both sides; keep the conflict visible; "do not select a winner silently."
- **Locked scripts (§36):** a participant "cannot use LAN access to bypass screenplay lock". A locked draft follows the normal post-lock revision workflow. Permissions apply and AI cannot bypass it (OC-047).

## 18. Conflict resolution UI (§17, §33)

Detailed panel (§17), exact labels:
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
- "The interface should identify the affected object and context." (OC-025)

Compact UX contract (§33), exact labels:
```text
Conflict Detected

Your version
Incoming version
Context

[Keep Mine]
[Use Incoming]
[Review Differences]
```
Note: §17 includes **[Manually Combine]** and a **Current working copy** pane. §33 omits both and adds **Context**. Implement the superset: Your version / Incoming version / Current working copy / Context, plus Keep Mine / Use Incoming / Review Differences / Manually Combine.

Resolution outcome: a clear resulting state plus an activity entry (§23.4). An activity event "conflict resolved" (§29).

---

## 19. Disconnect, reconnect and revalidation

### 19.1 Participant disconnect (§18.1)
- acknowledged shared changes remain in the session/project (OC-027);
- user identity remains in the project permissions model;
- presence changes to disconnected;
- unacknowledged local work is retained locally where possible (OC-028);
- soft locks held by the participant may be released (§15.3).

### 19.2 Host disconnect (§18.2)
- If the host loses the network or terminates unexpectedly, "connected participants must be informed that the shared session is unavailable."
- "They must not be told that the project has been deleted."
- The exact message text is [NOT SPECIFIED].

### 19.3 Local work preservation (§18.3)
"Disconnected local work should remain available for safe recovery/export where supported."

### 19.4 Offline-after-collaboration (§19)
"A participant who continues working after losing the session must not automatically overwrite the host later." Preferred path:
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
"This reuses the same safe exchange model as remote collaboration." (OC-029)

### 19.5 Reconnection (§20). Not "Automatically merge everything."
1. identify session;
2. identify last acknowledged state;
3. identify local unacknowledged changes;
4. compare with host/current state;
5. surface conflicts where present;
6. resume safe shared work or route divergent local work through review.

"No silent data loss." (OC-030)

### 19.6 Offline/reconnection matrix (§40)
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

## 20. Failure safety (§30)

- **Network failure cannot:** delete the project; overwrite the local project with an empty state; silently publish anything; convert the project to cloud dependency.
- **Storage failure:** never misreported as network failure. Show the storage problem and the recovery path.
- **Start failure:** solo work remains. **Join failure:** no mutation; explain; allow local work.
- **Conflict failure:** preserve both, keep visible, no silent winner.

---

## 21. What participants can and cannot do (consolidated)

**Can:**
- join using local session information; receive the assigned role; see the permitted shared scope (§12.3);
- edit content their role allows (Editor), or comment where allowed (Commenter), or view (Viewer);
- use AI read-only queries within their permitted scope directly (§26.1);
- propose AI mutations through the normal approval flow (§26.2);
- leave the session; keep local work; fall back to Exchange Packages (§24);
- explicitly convert their own private information into shared content through normal workflow (§10).

**Cannot:**
- see excluded private areas or other participants' private notes (§12.4, §10, §26);
- gain permanent access or permissions from the session (§13.1, §31, OC-039);
- enter live editing as Export-only (§8.6, OC-014);
- use AI to bypass their role, or to access "hidden host-only data; another participant's private notes; excluded workspaces; unauthorized objects" (§9, §26);
- bypass screenplay lock via LAN (§36);
- automatically overwrite the host with offline work (§19, OC-030);
- reverse-sync a Call Sheet edit into the Schedule (§34, OC-044);
- end the session or manage permissions (host/Owner only, §8.2, §12.2).

## 22. Source of truth during collaboration (§22, §34, §35, §44)

- "LAN collaboration does not create a second source of truth." The session "is not a second database."
  ```text
  Local Project
       ↑
  Collaboration Session
       ↑
  Participants
  ```
- Exchange Packages remain snapshots and are "not a live branch." Historical documents remain snapshots.
- "A review response is not automatically canonical." Disconnected local work "is not automatically canonical until safely reconciled."
- "A schedule edit remains a schedule edit. A screenplay edit remains a screenplay edit." Collaboration "must never collapse these distinct sources."
- **Call Sheets/Schedule (§34):** a schedule change does not silently rewrite issued call sheets (OC-045). A call-sheet refresh requires user review. Editing a Call Sheet does not change the schedule (OC-044). "Collaboration does not permit reverse synchronization that the normal product model forbids."

## 23. AI in offline and collaboration contexts (§25–§27)

- **Local AI** may run offline using: accessible local project data; locally available product knowledge; deterministic project query tools; local application tools.
- **External AI** needs a network. When unavailable, AI reports "that the external provider cannot be reached", core workflows continue, and local AI may still be available. The exact message is [NOT SPECIFIED].
- **No AI dependency:** core workflows are fully usable without AI (OC-038).
- **During collaboration:** AI uses the current participant's permissions and scope.
- **AI mutation flow (§26.2):** `Request → Context → Permission → Proposal → Preview → User acceptance → Normal project action`. "Collaboration does not weaken this rule." (OC-036)
- **Revalidation:** an AI Change Set built against an earlier collaboration state is revalidated before apply (§26.3).
- **External disclosure (§27):** the user receives the normal external-processing disclosure, reflecting the content sent. Only the authorized/requested context is transmitted (OC-037).
- **Offline import/export (§28):** if AI mapping is unavailable offline, deterministic/manual mapping remains available.

## 24. Activity and audit (§29)

Meaningful events may be recorded as Activity Entries:
- collaboration session started;
- participant joined;
- participant left;
- project data changed;
- conflict resolved;
- session ended;
- exchange package imported;
- exchange response applied.

"Presence itself need not create noisy permanent activity records." "The activity log is not a full network packet log." (OC-043)

## 25. No cloud dependency (§32)

None of the following requires OpenFrame cloud: ordinary local use; offline editing; local saving; local backups; local project packages; review Exchange Packages; LAN collaboration.

## 26. Data model dependencies (§38)

Canonical objects from Domain/Data: Application User; Exchange Package; Import Session; Collaboration Session; Snapshot; Change Set; Project; Project File; Deleted Item; Activity Entry.

## 27. Exact labels and messages index

| Context | Exact text |
|---|---|
| Offline shell | "Offline" |
| Save states | "Saved" / "Saving" / "Save Error" / "Saved with pending package operation" |
| Recovery prompt | "We found a recent recovery state for this project." |
| Recovery options | "Open Recovery" / "Keep Saved Version" / "Compare/Review" |
| Backup option | "Create Backup Before Import" |
| Collision options | "Open as Copy" / "Replace Existing After Backup" / "Cancel" |
| Drive-loss principle | "Do not manufacture a second truth because the original storage temporarily disappeared." |
| Host dialog | "Start Collaboration Session", "Project", "[Current Project]", "Shared Scope", "[Entire Project]", "[Story]", "[Screenplay]", "[Production]", "Participants / Roles", "[Start]" |
| Join dialog | "Join Local Session", "Session", "Role", "Shared Scope", "[Join]" |
| Active session | "Working in local shared session" |
| Presence | "Riya — editing" |
| Soft lock | "Riya is editing this scene." |
| Conflict (detailed) | "Conflict in Scene 24", "Your version", "Incoming version", "Current working copy", "[Keep Mine]", "[Use Incoming]", "[Review Differences]", "[Manually Combine]" |
| Conflict (compact) | "Conflict Detected", "Your version", "Incoming version", "Context", "[Keep Mine]", "[Use Incoming]", "[Review Differences]" |
| Start failure | "Solo local work remains available." |
| Same-text conflict | "Create a visible conflict." |
| Presence questions | "Who else is here?" / "Who is editing this?" |
| Role names | Owner / Editor / Commenter / Viewer / Export-only |
| Session states | Not Running / Host Starting / Running / Participant Joined / Participant Active / Participant Disconnected / Session Ended |
| Host states | Idle / Starting / Running / Participant Join/Leave / Ending / Ended |
| Participant states | Disconnected / Joining / Connected / Active / Rejoining |
| Conflict states | No Conflict / Conflict Detected / Review / Resolved |

"Riya" and "Scene 24" are example placeholders for participant name and object.

## 28. Acceptance criteria (§41)

| ID | Criterion |
|---|---|
| OC-001 | Project opens without internet |
| OC-002 | Core editing continues offline |
| OC-003 | Production workflows continue offline |
| OC-004 | Supported exports work offline |
| OC-005 | Supported local imports work offline |
| OC-006 | Backup can be created offline |
| OC-007 | Local project remains the primary persistence source |
| OC-008 | Offline state never requires cloud synchronization to open the project |
| OC-009 | External drive projects remain valid while the drive is available |
| OC-010 | Drive loss does not create a silent divergent project |
| OC-011 | Crash recovery can offer a recoverable state |
| OC-012 | User can choose recovery versus confirmed saved state |
| OC-013 | Collaboration roles are Owner, Editor, Commenter, Viewer, Export-only |
| OC-014 | Export-only user cannot enter live editing |
| OC-015 | Host can choose collaboration scope |
| OC-016 | Participants see only permitted shared scope |
| OC-017 | Private notes remain private during collaboration |
| OC-018 | LAN session has visible host/session state |
| OC-019 | Presence is visible where useful |
| OC-020 | Soft locks can protect high-conflict editing areas |
| OC-021 | Soft locks cannot permanently lock out a disconnected participant |
| OC-022 | Different-object edits coexist where safe |
| OC-023 | Independent same-object field edits can coexist where safe |
| OC-024 | Same-text-region conflicts become visible |
| OC-025 | Conflict UI provides enough context for user choice |
| OC-026 | No silent last-write-wins data loss occurs |
| OC-027 | Participant disconnect preserves acknowledged changes |
| OC-028 | Unacknowledged local work is retained where possible |
| OC-029 | Participant offline changes are reconciled through a safe review path |
| OC-030 | Reconnection does not blindly overwrite the host |
| OC-031 | Ending a LAN session does not upload/publish the project to OpenFrame cloud |
| OC-032 | Remote collaboration can fall back to Exchange Packages |
| OC-033 | Exchange Packages can be used without OpenFrame cloud |
| OC-034 | AI respects collaboration permissions |
| OC-035 | AI does not expose host-only or private participant data |
| OC-036 | AI mutations still require explicit user approval |
| OC-037 | External AI use is disclosed before project data is transmitted externally |
| OC-038 | Core product behavior remains functional when AI is disabled |
| OC-039 | Collaboration session state is temporary and does not become permanent project permission |
| OC-040 | Session start failure does not block solo work |
| OC-041 | Session join failure does not mutate local project content |
| OC-042 | Conflict resolution creates a clear resulting project state |
| OC-043 | Meaningful collaboration events can be represented through Activity History |
| OC-044 | Call Sheet changes do not reverse-sync into the Schedule |
| OC-045 | Schedule changes do not silently rewrite issued Call Sheets |
| OC-046 | Production Source staleness remains visible after collaborative screenplay changes |
| OC-047 | Locked screenplay protections remain active during LAN collaboration |

**Traceability (§42):**
- PRD: PRD-COL-001 (Permissions), PRD-COL-002 (Review Packages), PRD-COL-003 (Page-Specific Exchange Packages), PRD-COL-004 (Better Exchange Package Controls), PRD-COL-005 (Basic Local-Network Collaboration), PRD-COL-006 (Advanced Collaboration Conflict Handling), PRD-OFF-001, PRD-AI-001, PRD-AI-002.
- FSD: FSD-COL-001..005 (roles), FSD-COL-006..011 (package types), FSD-COL-012..016 (preview/stale/conflict/ambiguous/unmapped), FSD-COL-017..022 (LAN host/join/presence/conflict/disconnect/end), FSD-OFF-001..010 (FSD-OFF-010: drive loss), FSD-AI-001..012 (FSD-AI-012: failure safety).

## 29. Final safety model (§46)

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
"The goal is not to recreate a cloud collaboration service."

---

## 30. Gaps: [NOT SPECIFIED] in the source

| Area | Missing detail |
|---|---|
| Discovery | Mechanism (mDNS/Bonjour, UDP broadcast, manual address, join code/QR), firewall prompts, ports |
| Auth/pairing | How a participant proves identity; how the host binds an Application User to a role; session secrets/encryption |
| Participants/Roles UI | How the host pre-assigns roles (invite list vs approve-on-join) |
| Transport/sync | Wire protocol, op-log vs CRDT vs OT, acknowledgement protocol, ordering, batching |
| Soft-lock timeout | No lease duration or heartbeat interval; release is only on "known disconnect" or host action. The disconnect-detection timeout is also undefined |
| Request-access flow | UI and handshake for "wait/request access" |
| Which areas allow concurrent editing | "where safe" is undefined per object type |
| Text merge granularity | Definition of a "text region" (element, paragraph, character range) |
| Host-disconnect message | Exact text (only the constraints are given) |
| Join-failure message | Exact text |
| External-AI-unreachable message | Exact text |
| Participant local persistence | Where unacknowledged participant work is stored, and whether participants hold a full local copy of the shared scope |
| Rejoin rules | Time window, identity matching on rejoin |
| Session end UX | Confirmation dialog, participant notification text |
| Commenter/Viewer in LAN | Whether they may join live sessions (only Export-only is explicitly barred) |
| Owner vs Host | Whether the host must be the Owner (§8.2 gives session management to Owner; §12.2 to the host) |
| Canonical ↔ detailed state mapping | Two state vocabularies (§38.1 vs §39) with no explicit mapping |
| Conflict UI variants | §17 (4 buttons, "Current working copy") vs §33 (3 buttons, "Context") |
| Backup naming format | Only the elements are specified (project name, date/time, optional label) |
| Backup categories wording | "included/copied … unsupported" (OC §6.3) vs "skipped/unsupported" (IE §8.3) |
