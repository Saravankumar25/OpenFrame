# OpenFrame Studio — Functional Specification Document (FSD)

**Document type:** Functional Specification Document

**Product:** OpenFrame Studio

**Primary platform:** Windows + macOS desktop application

**Operating model:** Local-first, offline-capable, user-owned project files; optional internet-assisted AI and optional local-network collaboration; no mandatory OpenFrame cloud.

**Source PRD:** `OpenFrame_Studio_Mega_PRD_Aligned_Updated.md`

**Cross-cutting AI Specification:** `OpenFrame_Studio_AI_Specification_Updated.md`

**Purpose:** Define the exact functional behavior of OpenFrame Studio so product, UX, QA, and engineering teams can implement the product without needing to reinterpret the product vision for every feature.

**Functional scope:** Project creation, Global and Project Idea Vaults, Project Home, Story Board, Acts, Sequences, Beats, Scene Cards, Characters, Story Timeline, Screenplay Workspace, screenplay import/export, draft/version history, comparison, review rounds, comments, script lock and post-lock revisions, episodic organization, script breakdown, automatic breakdown suggestions with confirmation, production catalog, locations, cast/crew, moodboards, storyboards, shot lists, stripboard/scheduling, scheduling assistance, call sheets, project files, global search, lightweight budget snapshot, AI assistant, local/offline operation, portable exchange packages, local-network collaboration, permissions, undo/redo, deletion/recovery, saving, portability, backups, activity history, reports, sides, daily production view, templates, exports, printing, empty states, errors, and end-to-end source-of-truth rules.

**Explicit exclusions:** enterprise ERP, payroll, union accounting, full accounting, advanced studio resource planning, VFX production tracking, post-production asset management, distribution CRM, festival submission suite, legal-management suite, talent marketplace, equipment marketplace, and a mandatory cloud-sync platform.

---

## How to use this FSD

This document is intentionally more detailed than the PRD. The PRD defines product intent and scope. This document turns each in-scope capability into functional behavior. It describes:

1. where a user enters the feature;
2. what appears on screen;
3. what the user can do;
4. what state changes occur;
5. what information is created, retained, or copied;
6. which other modules are affected;
7. what must not happen automatically;
8. what happens on errors, cancellation, deletion, or import/export; and
9. how QA can verify the behavior.

Where the source PRD was intentionally flexible, this FSD uses the simplest behavior consistent with the stated product philosophy. Such choices are marked **FSD Decision**. They are product behavior decisions, not technical architecture.

> **Priority Rule:** All P0/P1/P2 classifications in this FSD inherit their priority from the PRD. The FSD cannot independently promote, demote or create product scope.

Every FSD requirement should contain:

```text
PRD Requirement ID
FSD Requirement ID
Priority inherited from PRD
Functional behavior
Acceptance criteria
```

# 1. Functional Contract and Non-Negotiable Rules

This section is the contract that applies to every feature below. A feature that violates one of these rules is considered functionally incorrect even if its individual UI works.

### 1.1 One application, multiple practical workspaces
OpenFrame is one desktop application with several workspaces. A filmmaker should be able to move between Idea Vault, Story, Screenplay, Breakdown, Production and Call Sheets without creating duplicate projects or manually copying ordinary metadata between screens.

### 1.2 Progressive structure
The least structured area is Idea Vault. The Story Board adds lightweight visual structure. The Screenplay adds strict writing structure. Breakdown and scheduling add production structure. This progression must be visible to the user.

### 1.3 No duplicate entry unless it is a deliberate copy
If information already exists in a source that should feed a downstream workflow, the system should reuse it or offer an import/conversion action. It must not require retyping the same scene heading or title solely to move forward.

### 1.4 No hidden synchronization
The Idea Vault is not a live synchronized database of screenplay content. The Story Board is a reference/outline surface, not an always-synchronized copy of the script. Downstream production data is derived from a selected screenplay/source version, but production changes should not silently rewrite the screenplay.

### 1.5 Experimentation must be safe
Creative users must be able to duplicate cards, create drafts, move cards to a Parking Lot, and undo actions without fear of destroying earlier work.

### 1.6 User control over automation
AI and automatic parsing can suggest. The user confirms anything that becomes meaningful project data. No AI operation may silently change story, script, breakdown, schedule, or call sheet information.

### 1.7 Offline ownership
Core workflows work without internet. The absence of internet cannot prevent a user from opening, editing, saving, or exporting a local project.

### 1.8 File exchange instead of mandatory cloud collaboration
Remote collaboration defaults to portable exchange packages. Real-time collaboration is an optional local-network session. OpenFrame does not require its own cloud service for ordinary use.

### 1.9 Short films are first-class projects
A short film can use only Story Board + Screenplay + a few production features. The software must not force empty enterprise-style sections into a short-film workflow.

### 1.10 Professional output, simple interaction
The app may produce professional-looking documents while keeping input forms short and approachable.

### 1.11 Explicit user decisions override suggestions
If an automatic suggestion conflicts with a user's manual decision, the manual decision remains authoritative.


# 2. Functional Terminology and Object Identity

The following terms must have one consistent meaning across the product.

| Object | Meaning | Is it authoritative? |
|---|---|---|
| Global Idea Vault Item | A loose creative item owned by the filmmaker and available across projects | Yes for the item itself |
| Project Idea Vault Item | A project-scoped creative item | Yes for the item itself |
| Act | A high-level Story Board container | Yes for Story Board structure |
| Sequence | A named container containing a group of story scenes/cards | Yes for Story Board grouping |
| Beat | A small story event/idea | Yes for the Beat object |
| Scene Card | A compact outline/reference representation of a scene | Yes for the card, not the screenplay text |
| Screenplay Scene | The actual scene inside the screenplay | Yes for written script content |
| Screenplay Draft | A named, user-recognized version of the screenplay | Yes for that draft |
| Automatic History Point | A recoverable background version | Restore source only; not a deliverable draft |
| Breakdown Element | A production need associated with a screenplay scene | Yes for production planning |
| Catalog Item | Reusable production item such as a prop/location/person | Yes for catalog identity |
| Shot | A planned camera/coverage unit | Yes for shot planning |
| Shooting Day | A scheduled production day | Yes for schedule |
| Call Sheet | A document prepared for a shooting day | Yes for that document; not automatically authoritative over schedule |
| Exchange Package | Portable package for review/import/merge | Snapshot of its source at export time |

**FSD Decision — internal identity vs display numbering:** scene cards and story objects must retain stable internal identities even when users do not see IDs or scene numbers. Display scene numbers are generated for the screenplay and production documents from ordered screenplay scenes; they are never manually maintained on Story Board cards.

### 2.1 Identity rule
Moving an object must not create a new object. Duplicating an object creates a new object. Importing an exchange package creates or updates objects according to its package rules; it must never accidentally treat a copy as the same identity unless the package explicitly represents a review/change to an existing object.


# 3. Application Shell and Global Navigation

### 3.1 Main shell
The shell remains stable while the central workspace changes. The user should be able to resize the window and have the active workspace adapt without opening many unrelated popups.

Default left navigation:

- Home
- Idea Vault
- Story
- Screenplay
- Breakdown
- Production
- Call Sheets
- Files

Top-level controls:

- current project selector;
- global search;
- save/sync/session indicator;
- Undo/Redo;
- Help/command access;
- user/project access controls.

### 3.2 Current project indicator
When a project is open, the project title must be visible in the persistent shell. If the user changes project, the shell title changes immediately after the new project becomes active.

### 3.3 Unsaved state
The interface must distinguish at minimum:

- Saved
- Saving
- Saved with pending external/package operation
- Save error

Core editing must continue during transient save work. If a save fails, the user must receive a visible but non-blocking explanation and a clear retry/recovery path.

### 3.4 Project switching
When switching projects with unsaved work, the application must complete save or ask the user whether to stay, save, or cancel the switch. It must never silently abandon changes.

### 3.5 Command consistency
Common actions use consistent labels and placement:

- New
- Open
- Duplicate
- Rename
- Delete
- Undo
- Redo
- Export
- Import
- Search
- Settings

Context-specific actions may appear in context menus.

### 3.6 Context menus
Right-clicking an object should expose only actions meaningful to that object. A Story Board scene card should not display payroll or call-sheet commands.


# 4. Project Creation and Project Lifecycle

### 4.1 New Project
New Project opens a minimal creation dialog.

Required:

- Title
- Project type: Feature Film / Short Film / Episodic / Series

Optional:

- Language
- Genre
- Creator

FSD Decision: no budget, schedule, team, cast, or location fields are required during project creation.

### 4.2 Creation result
After confirmation, the project is created and opened at Project Home. The project immediately contains empty workspaces and does not require setup wizard completion.

### 4.3 Project status
Supported high-level status labels:

- Idea
- Development
- Writing
- Rewrite
- Shooting Draft
- Pre-Production
- Shoot Preparation
- Shooting
- Archived

Status can be changed manually. The application may suggest status based on activity but must not automatically change the user's chosen project status.

### 4.4 Project duplicate
Duplicate Project creates a separate project package with copied project content. The new project must receive its own local identity. The UI should clearly show “Copy of …” until renamed.

### 4.5 Archive
Archive hides a project from normal recent-project views without deleting it. Archived projects remain openable via an Archived/All Projects view.

### 4.6 Restore
Restoring an archived project returns it to active projects without altering its contents.

### 4.7 Close project
Closing a project returns to the application home/recent projects view. It does not delete the project.


# 5. Idea Vault — Global and Project Workspaces

### 5.1 Purpose
Idea Vault is intentionally unstructured. The primary action is **Add**, not “classify.” The user may dump anything related to a film and postpone interpretation.

### 5.2 Supported items
The system must support at least:

- text notes;
- images;
- URLs;
- PDFs;
- office/document files;
- audio files and recorded voice notes;
- video files;
- folders/collections;
- handwritten/sketch images;
- quotes;
- screenshots;
- ordinary attached files supported by the host desktop environment.

### 5.3 Add flow
User selects Add and chooses an item type, or drags supported content into the Vault. The least amount of metadata necessary to store the item must be requested.

For text: open editor immediately.

For image/file: ingest/display item and allow optional title/note.

For URL: save the URL and user note; metadata preview may be obtained when available.

For voice note: record, stop, save; transcription is optional.

### 5.4 Untitled items
Untitled items are valid. The application may create a generated local display name for navigation, but that generated name is not treated as user-created content.

### 5.5 Views
The same Vault can be viewed as:

- Visual Grid
- Card View
- List View
- Folder View

View selection changes presentation only; it does not duplicate data.

### 5.6 Collections and folders
Folders are containers selected by the user. Collections may be used as thematic groups. A Vault item may be in a folder and additionally associated with tags/collections without being physically duplicated.

### 5.7 Optional metadata
Tags, pins, links to project objects, captions and notes are optional. The Add flow must never require them.

### 5.8 Global vs project Vault
The Global Vault is available outside any project. The Project Vault is scoped to the current project.

Copying an item from Global to Project creates a project-local copy. Editing the project copy does not modify the global original.

### 5.9 Vault search
Search can match titles, note text, captions, filenames and tags. Search should not require semantic classification.

### 5.10 Move to Story
A Vault item can be copied into Story as a Beat, Scene Card, Sequence idea, Character note, or generic Story note. The original remains in Vault. The generated Story object may show an informational “Source: Idea Vault” reference, but no live synchronization is established.

### 5.11 Vault deletion
Deleting a Vault item moves it to recoverable deleted items before permanent deletion. Permanent deletion requires deliberate confirmation.


# 6. Project Home and Continue Workflow

Project Home is the operational front door and must remain visually simple.

### 6.1 Required areas

**Project header:** title, type, status, last opened.

**Continue:** contextual shortcuts based on the user's latest activity.

**Quick Access:** Idea Vault, Story Board, Screenplay, Breakdown, Production, Call Sheets.

**Recent:** recently changed scenes/cards/drafts/documents.

**Project Files:** lightweight file cabinet.

### 6.2 Continue behavior
The application remembers the last meaningful location in the project. “Continue” returns there when possible.

Examples:

- Last open Scene Card → Story Board card opened.
- Last open screenplay scene → screenplay opens at that location.
- Last breakdown scene → breakdown opens that scene.
- Last open shooting day → schedule opens that day.

### 6.3 Empty project
A new project shows no fake metrics. It gives three high-value actions:

1. Open Idea Vault
2. Build Story
3. Write/Import Screenplay

### 6.4 Production-stage home
Once a project has production content, Project Home may additionally show:

- script status;
- breakdown progress;
- next shooting day;
- outstanding location/cast needs;
- latest call sheet;
- recent production document.

These are shortcuts, not a mandatory analytics dashboard.


# 7. Story Workspace and Story Board

### 7.1 Purpose
The Story Board is the filmmaker's visual reference and outlining surface. It is not a screenplay editor and not a database form.

### 7.2 Primary views

**Board View:** card-based visual arrangement.

**Outline View:** compact hierarchical text/list representation.

Both views represent the same Story Board structure.

### 7.3 Hierarchy
Default hierarchy:

```text
Act
  └── Sequence
       ├── Beat (optional)
       ├── Scene Card
       ├── Scene Card
       └── Scene Card
```

A Scene Card may also exist directly under an Act when the user does not need sequences. A Beat may exist independently in the Story Board/Parking Lot before being promoted to a Scene Card.

### 7.4 Board layout
Acts should be visually separated. Sequences appear as groups within Acts. Scene Cards appear as compact rectangular cards. The board should permit horizontal scrolling or zooming when a large number of cards exists.

### 7.5 Outline view
Outline view shows indentation and order. It is intended for precise hierarchy management, not additional metadata entry.

### 7.6 Board state preservation
Switching between Board and Outline views must not alter order or content.


# 8. Acts

### 8.1 Create Act
User selects Add Act. A new Act container appears at the end of the Story Board. User enters a title; optional note is available after creation.

### 8.2 Act behavior
An Act can contain Sequences and/or Scene Cards. An Act can be collapsed/expanded.

### 8.3 Rename Act
Double click title or context menu → Rename. Rename updates everywhere the Act is referenced by Story Board displays or exports, but does not alter screenplay text.

### 8.4 Reorder Act
Drag Act to another position. All child sequences/scenes move with it.

### 8.5 Delete Act
Delete requires confirmation if it contains child objects. User can choose to:

- delete the Act and its children;
- move children to another Act/Unassigned;
- cancel.

FSD Decision: default confirmation should recommend moving children out rather than deleting creative material.


# 9. Sequences

### 9.1 Purpose
A Sequence is deliberately simple: it is a named container for a group of scenes. It is not a formal production object.

### 9.2 Create Sequence
User selects Add Sequence under an Act. The primary field is a text field.

Examples:

- Railway Station Scenes
- Investigation
- Hero Introduction
- Chase
- Family Confrontation

No required “purpose” field exists.

### 9.3 Child content
The normal child content is Scene Cards. Beats may also temporarily exist inside the sequence.

### 9.4 Reorder
Dragging a sequence changes its position inside the Act. All contained scene cards move with it.

### 9.5 Move between Acts
Dragging a sequence to another Act re-parents the sequence. Existing scene identities remain unchanged.

### 9.6 Delete
Deleting a Sequence with children presents a safe choice: delete the container only after moving children elsewhere, or delete all with explicit confirmation.

### 9.7 Sequence as reference only
A sequence does not become a screenplay sequence automatically. Its name is a Story Board organizational aid unless the user chooses to include that hierarchy in an outline export.


# 10. Beat Cards

### 10.1 Purpose
A Beat Card represents a small story event, idea, turn, or moment that may or may not become a scene.

### 10.2 Minimum content
One text field: Beat text.

Optional:

- note;
- color;
- attachment.

### 10.3 Creation
User can create a Beat in a sequence, Act, or Parking Lot.

### 10.4 Drag behavior
A Beat can move anywhere the board accepts a Beat. Dragging changes order and parent. No version branch is created.

### 10.5 Convert Beat to Scene
The user may choose Convert to Scene. OpenFrame creates a new Scene Card using the Beat text as its initial short description. The original Beat may be retained as a history/reference object or marked converted according to UI choice; the default is to retain it in a “Converted” reference state so no creative thought is lost.

### 10.6 Beat not used
A Beat can remain in Parking Lot indefinitely without impacting the screenplay.


# 11. Scene Cards — Exact Functional Behavior

### 11.1 Design target
Scene Cards are compact rectangular boxes designed to fit many on screen. A card is primarily a short description and an optional scene header.

### 11.2 Minimum content
Required at creation: short description (may initially be blank while entering a new card).

Optional: scene heading such as `INT. POLICE STATION — NIGHT`.

Not stored as Story Board fields:

- scene number;
- characters;
- story day;
- production day;
- props;
- wardrobe;
- breakdown categories;
- budget;
- shot list.

Those are derived or managed later.

### 11.3 Card display
Collapsed card shows:

- short description;
- optional small indicator that a header exists;
- small status indicators only when useful (e.g., comments count) and never a metadata wall.

Long descriptions truncate visually without truncating the stored text.

### 11.4 Expanded scene card
Double click opens expanded card/drawer.

Fields:

- optional scene heading;
- short description;
- freeform scene notes;
- attachments;
- comments.

### 11.5 Create
A new card is inserted at the drop position or end of selected sequence. The description is immediately editable.

### 11.6 Reorder
Drag and drop changes Story Board order instantly. The system records the move for undo.

### 11.7 Duplicate
Duplicate creates a new Scene Card with copied description, optional heading, notes and selected attachments. The duplicate is not the same object and does not inherit a screenplay scene identity.

### 11.8 Delete
Delete moves the card to recoverable deleted state. If the card has a screenplay relationship, deletion must warn that the linked screenplay scene is not automatically deleted.

### 11.9 Parking Lot
Dragging a card into Parking Lot removes it from the active outline but keeps it recoverable.

### 11.10 No live screenplay synchronization
Changes to a Scene Card after screenplay creation do not rewrite the screenplay scene. The Story Board may display a simple “used to create screenplay” reference without becoming a synchronization engine.

### 11.11 No permanent scene number
Scene numbers are calculated from screenplay scene order. Reordering a Story Board card does not require manual renumbering because the card has no user-maintained number.

### 11.12 Heading required at screenplay conversion
Because the Story Board allows an optional header, Build Screenplay must identify any included cards without a valid screenplay heading and request that the user supply the missing heading before the scene can be created as a normal screenplay scene. The user can also exclude those cards from the build.


# 12. Parking Lot and Creative Experimentation

Parking Lot is a simple holding area, not a branch-management engine.

### 12.1 Add to Parking Lot
Any Beat or Scene Card may be moved into the Parking Lot.

### 12.2 Restore
Drag back into an Act/Sequence or choose Restore to Story.

### 12.3 Experimentation model
The primary ways to experiment are:

- duplicate a Beat/Scene Card;
- reorder;
- move to Parking Lot;
- make a named screenplay draft.

There is no multi-branch story graph, branching state tree, or alternate-ending comparison subsystem in the initial product.

### 12.4 Visual distinction
Parking Lot cards should look slightly different so the user understands they are not currently part of the active story order.


# 13. Characters

### 13.1 Purpose
Characters are a simple project directory used primarily for story reference and later production linkage.

### 13.2 Create character
Fields:

- Name (required)
- Optional short description
- Optional role label
- Optional image
- Optional notes

### 13.3 Character references
A character page should show screenplay scenes where the character appears after screenplay parsing identifies them. The user can correct false detections.

### 13.4 Manual character link
The user may associate a character with a Scene Board card for reference, but this is optional and not required for a scene card to exist.

### 13.5 Rename character
Renaming the character record must not blindly change arbitrary text in screenplay dialogue/action. The screenplay editor has its own text and replacement controls. Character identity references used in production can be updated as relationships change.

### 13.6 Delete character
If the character is used in screenplay/production data, deletion requires confirmation and offers archive/remove-from-directory behavior rather than destructive removal by default.


# 14. Story Timeline

Story Timeline is optional. It exists only when chronological continuity matters.

### 14.1 Timeline values
The simplest unit is Story Day, optionally supplemented by time-of-day notes.

### 14.2 Assignment
Users can assign story days to screenplay scenes after scenes exist. Story Board cards do not require Story Day.

### 14.3 Display
Timeline shows ordered scenes by story day. If multiple scenes are unassigned, an “Unassigned” group appears.

### 14.4 Conflict notification
If a user assigns a later event to an earlier day than a referenced dependent event, the system may display a soft continuity warning. It must never reorder scenes automatically.

### 14.5 Episodic timeline
For episodic projects, story day may be episode-scoped or series-scoped according to project setting. The default should be episode-scoped to avoid unnecessary complexity.


# 15. Screenplay Workspace — Overall Behavior

### 15.1 Purpose
The Screenplay Workspace is the authoritative written document area.

### 15.2 Entry paths
Users can enter it by:

- Create Screenplay
- Open current screenplay
- Open a draft
- Import screenplay
- Build from Story Board

### 15.3 Modes
- Focus Mode: screenplay page dominates the window.
- Standard Mode: scene navigator and compact tools visible.
- Writing Room Mode: screenplay plus optional Story Board / scene notes / character / comments panel.

### 15.4 Screenplay authority
The screenplay owns the written scene content. Production breakdown uses a chosen screenplay/source version. Story Board remains a reference layer unless the user explicitly uses a board action to create/reorder screenplay content.

### 15.5 Basic writing controls
Required:

- scene heading;
- action;
- character;
- dialogue;
- parenthetical;
- transition;
- optional shot direction;
- user notes outside the printed script.

### 15.6 Automatic formatting
The editor should automatically apply screenplay formatting when the user moves between common element types. The writer must be able to override element type when needed.

### 15.7 Navigation
Scene navigator lists screenplay scenes using generated scene numbers and headings. Clicking a scene jumps to it without opening another document.

### 15.8 Search
Standard text search within the open screenplay supports find, next, previous, replace, case sensitivity and whole-word matching. No semantic “story search” is required.


# 16. Screenplay Writing Interactions

### 16.1 Typing
Text editing should behave predictably like a professional writing application. Cursor movement, selection, copy/paste, undo/redo and common text shortcuts must work without requiring a custom interaction model.

### 16.2 Element switching
The writer can change element type from a compact selector and keyboard shortcut. Contextual automatic selection may suggest the next likely type but must remain overridable.

### 16.3 Scene creation
New scene can be inserted from scene navigator or within the editor. A new scene receives its display number based on position; the user does not type the number.

### 16.4 Page formatting
The application maintains standard screenplay page layout for PDF/printed output while allowing the writer to work continuously without manually inserting page breaks for normal text.

### 16.5 Title page
Title page supports project title, author/creator, contact fields and optional revision/draft information according to export configuration.

### 16.6 Notes
Scene notes are outside the printed screenplay. Notes can be shown/hidden during writing and are excluded from normal screenplay PDF export unless the user explicitly chooses a notes export.


# 17. Writing Room Layout

The Writing Room is a layout, not a separate data model.

### 17.1 Default layout
Left: scene navigator.

Center: screenplay.

Right: optional panel.

### 17.2 Optional right panels
The user can open:

- Story Board reference
- Scene Notes
- Characters
- Comments
- Draft information

Only one or two panels should normally be visible at once.

### 17.3 No permanent Idea Vault sidebar
Idea Vault is accessed through the normal application navigation and search. It is not a synchronized writing-room panel.

### 17.4 Layout persistence
The application remembers the last layout for the project/user. Opening a script does not reset the user's preferred panel arrangement.

### 17.5 Panel independence
Opening a panel must never edit the underlying content merely because it is visible. For example, viewing a Story Board card does not lock or modify the screenplay.


# 18. Build Screenplay From Story Board

### 18.1 Entry
Button location: Story Board toolbar → Build Screenplay.

### 18.2 Preconditions
The user may select all active scene cards or a subset. Scene cards without screenplay headings must be resolved before conversion unless the user chooses to exclude them.

### 18.3 Preview
Before creating the screenplay, show a simple ordered preview:

| Order | Story Board scene | Heading | Include |
|---|---|---|---|

The user can deselect cards.

### 18.4 Build action
When confirmed:

1. Create a screenplay draft from the selected cards.
2. Create screenplay scenes in the same order.
3. Carry over the heading.
4. Place the description as a planning note or opening action placeholder according to the chosen creation mode.
5. Do not copy unrelated Idea Vault metadata.

### 18.5 Repeat build
Building again from the Story Board must not silently overwrite an existing screenplay. Options:

- create new screenplay draft;
- build new screenplay document;
- cancel.

### 18.6 Subsequent board changes
Board changes are not automatically applied to the script. The user may intentionally reorder or rebuild; the application must warn if the operation could affect written screenplay order.


# 19. Screenplay Import — Functional Requirements

### 19.1 Supported sources
- PDF
- Final Draft FDX
- Fountain
- TXT
- DOCX
- Pasted screenplay text

### 19.2 Import modes
User chooses one of:

- New screenplay
- New draft in current screenplay project

Never overwrite an existing draft by default.

### 19.3 Pre-import preview
The application should display detected:

- scenes;
- character names;
- approximate page count;
- document title;
- obvious parsing warnings.

### 19.4 PDF import
The product should attempt screenplay structure recognition from formatted PDF. If confidence is insufficient, it should show a review state and allow the user to continue with editable text rather than pretending the parse is perfect.

### 19.5 DOCX import
The product should extract readable text and formatting where possible, then map it to screenplay elements. Unsupported formatting is not silently treated as screenplay structure.

### 19.6 TXT/paste import
The user receives a parsing preview and can correct scene headings or element interpretation before confirming.

### 19.7 FDX import
FDX should preserve screenplay content and supported draft/revision metadata. Unsupported FDX properties should be ignored gracefully and reported in import summary if relevant.

### 19.8 Import report
After import, show:

- imported scenes;
- detected characters;
- pages/length;
- warnings requiring attention;
- source filename.

### 19.9 Source preservation
Original imported file can be retained in Project Files if the user chooses. The imported screenplay remains the editable OpenFrame object.


# 20. Screenplay Export

### 20.1 Required exports
- PDF
- FDX
- Fountain
- DOCX

### 20.2 Export scope
User can export:

- entire current draft;
- selected scenes where supported;
- clean script;
- revision-marked version when applicable.

### 20.3 PDF
PDF output must be professionally formatted and printable. Notes are excluded unless explicitly selected.

### 20.4 FDX
Export must preserve screenplay structure and content that maps to FDX. Unsupported OpenFrame metadata is retained internally but does not block export.

### 20.5 Fountain
Export must represent screenplay structure using standard Fountain conventions. Internal project metadata that is not part of screenplay text is not inserted into Fountain unless the user selects an optional header/comment export.

### 20.6 DOCX
Export produces an editable document suitable for ordinary document workflows while preserving readable screenplay layout.

### 20.7 Export completion
After export, the application displays the file location and offers “Reveal in File Manager.”

### 20.8 Export immutability
Exported files are snapshots. Editing an exported file outside OpenFrame does not change the project.


# 21. Drafts, Automatic History, and Version Control

### 21.1 Two layers
OpenFrame uses:

1. Automatic recoverable edit history.
2. Named drafts created intentionally by the user.

### 21.2 Automatic history
Automatic history exists for restoration and crash recovery. It is not treated as a deliverable script version.

### 21.3 Named draft creation
New Draft copies the selected source draft into a new version. User provides a name and optional note.

### 21.4 Draft lineage
Each named draft records its parent draft. The interface should show a simple lineage:

`Draft 1 → Draft 2 → Director Rewrite → Shooting Draft`

### 21.5 Current draft
Exactly one draft is marked Current within the screenplay workspace at a time.

### 21.6 Distribution copy
The user can export a clean distribution copy from any named draft. Export does not make that draft current.

### 21.7 Restore old draft
User may create a new draft “restored from Draft 2” rather than replacing the current draft.

### 21.8 Rename draft
Renaming changes only the draft label, not its text.

### 21.9 Delete draft
Deleting a named draft requires confirmation and moves it into recoverable deleted state. The current draft cannot be permanently deleted without first selecting another current draft.


# 22. Draft Comparison

### 22.1 Compare entry
From Draft History, select two named drafts → Compare.

### 22.2 Comparison modes
Two layers:

- Change summary by scene.
- Exact text comparison within a selected scene.

### 22.3 Scene-level summary
Display:

- added scenes;
- removed scenes;
- scenes moved;
- scenes with changed text;
- unchanged scenes.

### 22.4 Text comparison
Within a selected scene, show side-by-side or inline difference markers. Added text and removed text must be visually distinguishable.

### 22.5 No automatic merge
Comparison is read-only. User returns to a draft and edits manually or creates a new draft.

### 22.6 Scene identity during comparison
When a scene exists across drafts, the comparison should match by screenplay scene identity where available and fall back to heading/order heuristics for imported legacy documents. Ambiguous matches should be flagged rather than silently assumed.


# 23. Review Rounds and Comments

### 23.1 Review round creation
User selects a draft → Start Review. The review record stores source draft, reviewers, date, optional deadline, and status.

### 23.2 Comment targets
Comments can attach to:

- screenplay text;
- scene card;
- beat;
- sequence;
- act;
- storyboard panel;
- shot;
- location;
- breakdown element.

### 23.3 Comment lifecycle
Open → In Discussion → Resolved.

Resolved comments remain accessible in review history.

### 23.4 Comment text anchoring
Screenplay comments should remain associated with the intended text region where possible. If later editing makes the exact text unavailable, the comment becomes “context moved” and links to the nearest surviving scene rather than disappearing.

### 23.5 General scene comments
A general scene comment has no text-range anchor and remains attached to the scene object.

### 23.6 Permissions
Writer/Director/Producer/Reviewer may comment according to project role and sharing configuration. Viewer cannot add comments unless explicitly granted Commenter capability.

### 23.7 Review completion
A review can be marked Complete only by a user with appropriate project permissions. Completing a review does not automatically lock the script.

### 23.8 Comment export
Review packages may include comments. Normal screenplay PDF exports exclude internal comments.


# 24. Script Lock and Production Revisions

### 24.1 Lock action
User opens Draft History → selects a named draft → Lock as Shooting Draft.

### 24.2 Lock confirmation
Show concise confirmation containing:

- draft name;
- current draft status;
- open review comments;
- last modified date;
- scene count.

The application must explicitly state that future edits will create a post-lock revision flow.

### 24.3 Locked state
The locked draft remains readable/exportable. Editing begins through “Start Revision.”

### 24.4 Start revision
Creates a new revision based on the locked source. The locked source remains unchanged.

### 24.5 Revision labels
Support production labels such as:

- Revision A
- Revision B
- Blue Revision
- Pink Revision
- Yellow Revision

Exact naming can be chosen by the production.

### 24.6 Revision colors
The color applies to revision metadata and supported export presentation. The user can choose another color if the production uses a custom convention.

### 24.7 Changed scenes
Revision view lists scenes affected by the revision. User can inspect exact changes.

### 24.8 Production source
A production breakdown may select the locked shooting draft or a later approved revision as its source. The chosen source is recorded and visible.


# 25. Episodic Projects

### 25.1 Project hierarchy
Series project → Seasons → Episodes.

### 25.2 Episode creation
User can create an Episode with:

- episode number/order;
- title;
- optional one-line summary;
- status.

### 25.3 Each episode
Each episode has its own Story Board, screenplay drafts, breakdown and production planning.

### 25.4 Series-level references
Series-level lightweight directories may contain recurring characters and locations. Episode-level records can reference them.

### 25.5 Episode continuity
The application can show where recurring characters and locations appear across episodes. It does not need a large continuity-management database.

### 25.6 Season board
A Season Board displays episode cards that can be reordered by drag/drop.

### 25.7 Episode duplicate
Duplicate an episode with optional copy of its story/screenplay/production content. New episode identity is created.


# 26. Breakdown Workspace

### 26.1 Entry
User selects a screenplay source → Breakdown.

### 26.2 Scene list
Left side lists screenplay scenes. Selecting a scene opens script content and breakdown elements.

### 26.3 Categories
Default categories exactly:

1. Cast
2. Extras / Background
3. Location / Set
4. Props
5. Wardrobe
6. Vehicles
7. Hair / Makeup
8. Special Effects
9. VFX
10. Sound
11. Animals

### 26.4 Manual tagging
User highlights/selects text or creates an element manually. They choose category, then existing catalog item or new item.

### 26.5 Element assignment
If a catalog item already exists, selecting it reuses the same production identity. If new, the item is added to the production catalog.

### 26.6 Scene-level display
Breakdown view shows only relevant categories for the selected scene. Empty categories remain collapsed or hidden.

### 26.7 Remove element
Removing an element from a scene must not delete the catalog item itself. It only removes the scene association.

### 26.8 Catalog deletion
Deleting a catalog item requires handling all scene associations. Default behavior is archive, not destructive removal.


# 27. Automatic Breakdown Suggestions

### 27.1 Trigger
When a scene is opened in Breakdown, user can choose “Suggest Elements.” Suggestions may also be offered after import or when a scene is newly selected.

### 27.2 Suggestion output
Display grouped suggestions with category and matched text.

Example:

> **Prop — pistol** — matched phrase: “carrying a pistol.”

### 27.3 User actions
Each suggestion supports:

- Accept
- Reject
- Edit

### 27.4 Accept
Accepting creates or links the relevant production element. If a matching catalog item already exists, offer it as the first choice.

### 27.5 Reject
Reject removes the suggestion from the current scene. It may remain dismissed for that scene/source revision.

### 27.6 Edit
User can change category, name or association before accepting.

### 27.7 Confidence
The interface may show a plain-language confidence indicator if useful, but must not make the user decipher model scores.

### 27.8 No silent mutation
AI/automatic suggestions never become confirmed breakdown items without user action.


# 28. Production Catalog

### 28.1 Purpose
Catalog contains reusable production identities derived from breakdown and created manually.

### 28.2 Catalog categories
At minimum:

- Cast
- Locations
- Props
- Wardrobe
- Vehicles
- Makeup/Hair
- Special Effects
- VFX
- Sound
- Animals
- Extras

### 28.3 Catalog item
Minimum fields:

- Name
- Category
- Status

Optional:

- Description
- Image
- Notes
- Contact/reference

### 28.4 Usage view
Every catalog item can display “Used in Scenes.” Clicking a scene opens its breakdown.

### 28.5 Duplicate handling
When adding a new breakdown item, the interface should search for likely existing catalog matches by name/category before creating a duplicate. User confirmation is required for ambiguous matches.

### 28.6 Status
Useful lightweight values:

- Required
- Searching
- Shortlisted
- Confirmed
- Not Required

The exact set can be reduced in UI if a project is simple.


# 29. Locations

### 29.1 Create location
Fields:

- Name
- Area/address
- Contact
- Status

Optional:

- Photos
- Access notes
- Parking notes
- Noise notes
- Power notes
- Permission notes
- Travel notes

### 29.2 Scene usage
Location page shows scenes requiring the location.

### 29.3 Status
Idea → Shortlisted → Confirmed → Rejected.

### 29.4 Replacement
If Location A is rejected, user can set Location B as the practical replacement for selected scenes. This must update production associations, not screenplay text, unless the screenplay contains an explicit location name that the user separately edits.

### 29.5 Photos
Photo attachments remain accessible offline and are available for mood/reference export.


# 30. Cast and Crew

### 30.1 Cast record
Required:

- Person name
- Character association

Optional:

- Contact
- Photo
- Availability notes
- General notes

### 30.2 Crew record
Required:

- Person name
- Role/department

Optional contact and notes.

### 30.3 Character-to-actor assignment
One character may have one primary actor by default. Additional performers can be represented when necessary, such as double casting or alternates, but the UI should remain simple.

### 30.4 Availability
Availability is note-based/lightweight. There is no HR or payroll engine.

### 30.5 Scene involvement
From Cast, user can see scenes involving the character. From a scene, user can see required cast.

### 30.6 Conflict check
When scheduling a shoot day, if the same cast member is required in conflicting places/times according to entered data, the schedule shows a warning.


# 31. Moodboards

### 31.1 Board creation
User clicks + Moodboard, enters board name, then adds images/notes/links.

### 31.2 Common board names
The app may suggest:

- Overall Look
- Cinematography
- Production Design
- Costume
- Lighting
- Character
- Location

These are suggestions only.

### 31.3 Canvas behavior
Items can be dragged, resized and reordered. The board is freeform but must remain simple enough for ordinary project references.

### 31.4 Captions
Each visual item may have a short caption explaining why it matters.

### 31.5 Export
Export creates a clean PDF/image presentation excluding internal/private notes unless selected.


# 32. Storyboards

### 32.1 Purpose
Storyboard is a visual planning tool for shots, not professional illustration software.

### 32.2 Create storyboard
From Production → Storyboards or directly from a scene/shot list.

### 32.3 Panel contents
Required:

- visual panel/image/sketch placeholder;
- shot number;
- short description.

Optional:

- framing;
- camera movement;
- angle;
- dialogue/sound note;
- duration;
- storyboard note.

### 32.4 Add visual
User can:

- import image;
- draw/sketch if supported;
- use blank placeholder.

### 32.5 Reorder
Drag panels to change order. Shot numbers are recalculated from order for that scene/storyboard.

### 32.6 Scene association
A storyboard can be associated with a screenplay scene. It must not require the screenplay to contain shot directions.

### 32.7 Export
Export a clean storyboard sheet/PDF with optional shot notes.


# 33. Shot Lists

### 33.1 Purpose
Shot List is a practical camera-planning document built scene by scene.

### 33.2 Create shot
User selects scene → Add Shot.

Required:

- short description.

The application calculates/display shot number from order.

Optional:

- shot size/framing;
- movement;
- angle;
- lens;
- camera notes;
- characters;
- storyboard image;
- sound note.

### 33.3 Drag reorder
Dragging shots updates their order and displayed identifiers.

### 33.4 Storyboard connection
User can attach an existing storyboard panel or create a new panel from a shot.

### 33.5 Reference image
A shot may contain a reference image. The image is visual aid, not an asset-management relationship.

### 33.6 Export
Export by scene, group of scenes, or whole project.


# 34. Script Breakdown to Shot/Storyboard Workflow

The user may enter visual planning from any of these paths:

**Path A:** Screenplay scene → Shot List → Storyboard.

**Path B:** Screenplay scene → Storyboard → Shot List.

**Path C:** Screenplay scene → Shot List only.

No path requires the other module. The purpose is flexibility for filmmakers with different planning styles.

### 34.1 Create from scene
Selecting a screenplay scene in Production should offer quick actions:

- Breakdown
- Shot List
- Storyboard
- Moodboard reference

### 34.2 No duplicate scene entry
Scene heading and scene identity come from the screenplay source when a production object is created from it.

### 34.3 Changes after script revision
Shot and storyboard records are linked to scene identity, not raw line positions. If scene content changes, the app flags the scene for user review rather than deleting visual planning.


# 35. Stripboard / Shooting Schedule

### 35.1 Purpose
Stripboard translates screenplay scenes into practical shooting days.

### 35.2 Initial state
When a schedule is created from a screenplay source, all scenes enter **Unscheduled**.

### 35.3 Scene strip
Each strip displays:

- generated screenplay scene number;
- INT/EXT;
- location;
- day/night;
- page count;
- synopsis/description;
- optional cast indicators;
- important breakdown indicators.

### 35.4 Create shooting day
User adds a shooting day with:

- date;
- day number;
- optional notes.

### 35.5 Drag scene to day
Dragging a strip from Unscheduled into a day schedules it. The day updates its scene count, estimated time, cast requirements and location summary.

### 35.6 Reorder within day
Dragging a strip changes its production order for that day.

### 35.7 Move between days
Dragging to another day reassigns the scene. Any call sheet generated from the affected day becomes marked as needing update if it already exists.

### 35.8 Unscheduled state
A scene may remain unscheduled indefinitely. The user can filter to show only unscheduled scenes.

### 35.9 Day breaks
User can insert meal break, travel, company move or custom note markers.

### 35.10 No forced optimizer
Schedule assistance is advisory. Human arrangement remains authoritative.


# 36. Scheduling Assistance and Conflict Warnings

### 36.1 Suggestions
The application may suggest practical groupings based on visible production information such as shared location, cast, or entered shooting duration.

### 36.2 Example
If Scene 12, 18 and 21 use the same confirmed location, the system may suggest grouping them.

### 36.3 Accept suggestion
Accepting a suggestion moves scenes according to the proposed plan only after explicit confirmation.

### 36.4 Conflict detection
Warnings include:

- same actor in different locations on same day;
- required location scheduled in two incompatible places on same day;
- scene assigned to a day missing required location information;
- call sheet generated from a day that has changed since the last export;
- estimated time exceeds entered day duration.

### 36.5 Warning behavior
Warnings are non-blocking unless a user explicitly enables strict validation. Default is advisory.

### 36.6 Manual override
Every warning has a “Keep Anyway” path.


# 37. Schedule Calendar and Daily Plan

### 37.1 Calendar
Calendar view shows shooting dates and off days.

### 37.2 Date assignment
User can change a shooting date from day details. The date is part of the Shooting Day record.

### 37.3 Daily summary
Each day displays:

- scenes;
- locations;
- cast;
- estimated duration;
- notes;
- call sheet status.

### 37.4 Day duplication
A shooting day can be duplicated to create a similar planning template, but scenes themselves retain identity.

### 37.5 Rescheduling
Changing a day date updates the shooting day and marks its call sheet as needing refresh if it was already exported/prepared.


# 38. Call Sheets

### 38.1 Purpose
A call sheet is a practical daily document generated from a shooting day and then finalized manually.

### 38.2 Create
From schedule: select Shoot Day → Create Call Sheet.

### 38.3 Default sections

**Production:** title, date, shooting day number, crew call.

**Cast:** actor, character, call time.

**Scenes:** scene number, heading, short description.

**Location:** name, address.

**Practical:** parking, meeting point, travel notes, meal/break, emergency contact, production notes.

**Optional:** weather, reference images, attachments, special notes.

### 38.4 Minimal default
The editor should hide optional sections until added. User can expand the call sheet with extra fields if needed.

### 38.5 Pre-population
Scene, cast and location information comes from the shooting day and underlying project data. The user should not re-enter it.

### 38.6 Manual edits
The user may edit call time, notes, meeting point, attachments and other document-specific fields.

### 38.7 No reverse sync by default
Editing the call sheet does not alter the main schedule. If the user changes the schedule after creating a call sheet, the call sheet shows “Schedule changed — Refresh” rather than silently rewriting the document.

### 38.8 Finalize
User may mark a call sheet Final/Issued. This creates a clear document state but does not prevent later replacement with a new revision.

### 38.9 Export
PDF is required. The call sheet is a snapshot suitable for sending through ordinary communication channels.


# 39. Lightweight Budget Snapshot

This product includes only a lightweight budget view because the user selected a basic budget capability, not full production accounting.

### 39.1 Purpose
Help an indie filmmaker see the rough scale of a project and a few category totals.

### 39.2 Structure
Simple categories such as:

- Cast
- Crew
- Locations
- Equipment
- Art/Props
- Travel/Transport
- Food
- Post/Other
- Contingency

### 39.3 Data entry
Each category supports a small list of line items with description and amount.

### 39.4 Summary
Display:

- planned total;
- optional contingency;
- current entered total.

### 39.5 Explicit boundary
No payroll calculations, tax, union rules, accounting ledgers, purchase orders, invoices, cost reports, or enterprise cost codes.

### 39.6 Connection to project
Budget is advisory project information. A screenplay change may prompt the user to review budget but must not automatically change a monetary value without user confirmation.


# 40. Project Files

### 40.1 Purpose
Files is a project-level file cabinet for documents not represented by another module.

### 40.2 Supported operations
- Add file
- Rename display name
- Move to folder
- Open externally
- Reveal in file manager
- Export/copy
- Delete to recoverable state

### 40.3 No duplicate media system
If an image is already attached to a Moodboard, the project Files view should not create an unnecessary second user-visible copy merely because the file is referenced elsewhere.

### 40.4 File reference behavior
The user must be able to distinguish a file physically stored in the project from a link/reference to an external file where supported.

### 40.5 External modifications
If an externally referenced file changes, the user receives a refresh/relink warning when the relevant module is opened.


# 41. Global Search

### 41.1 Scope
Global search covers the current project by default and can optionally search the Global Idea Vault.

### 41.2 Searchable objects
- Ideas
- Acts
- Sequences
- Beats
- Scene Cards
- Screenplay text
- Characters
- Locations
- Catalog items
- Cast/Crew
- Shots
- Storyboards
- Call sheets
- Project files
- Comments

### 41.3 Result behavior
Each result displays type and context.

Example:

`red motorcycle — PROP — Production`
`red motorcycle — Scene 25 — Screenplay`
`red motorcycle — Idea Vault — Visual Reference`

### 41.4 Search actions
Click result → open the owning page and focus the matching item when possible.

### 41.5 No semantic search requirement
Normal text detection is sufficient for the core product.


# 42. AI Assistant

### 42.1 Purpose

AI is an optional system-wide natural-language assistant and command interface over the existing OpenFrame application.

It is not a second database and it is not an autonomous filmmaker.

The language model interprets requests, retrieves or requests the appropriate application operation, and explains results. Canonical project data, deterministic calculations, validation, permissions and mutations remain application responsibilities.

### 42.2 Availability

Core application functions continue to work without AI.

A configured local model may operate without internet. External/cloud AI providers may require network access.

### 42.3 Product knowledge and project knowledge

The assistant can use two categories of knowledge:

- **Product knowledge:** OpenFrame terminology, supported workflows, source-of-truth rules, permissions, versioning, import/export, offline behavior and supported commands.
- **Project knowledge:** user-accessible content from the current project and explicitly selected Global Idea Vault content.

The assistant should not rely on model memory when an authoritative application rule or project value is available.

### 42.4 Scope selector

Supported scopes include:

- Current selection
- Current scene
- Current screenplay
- Specific draft
- Story Board
- Selected Idea Vault items
- Production
- Specific shooting day
- Call Sheet
- Whole project
- Explicitly requested supported combinations

The resolved scope must be visible when it materially affects the result.

### 42.5 Read-only questions and exact project facts

AI may answer questions without changing data.

Examples:

- “Summarize this scene.”
- “Which scenes contain the police station?”
- “What changed between these drafts?”
- “What locations are used in Act 2?”
- “How many characters are in Draft 8?”
- “How many scenes are unscheduled?”
- “How many locations are used in the current screenplay?”

Exact counts, statistics and structured project facts must use deterministic application queries/calculations. The model must not guess exact values.

### 42.6 Natural-language navigation and commands

AI may execute explicit read-only or navigation commands such as:

- open a workspace;
- open a scene;
- open a draft;
- show search results;
- apply supported filters;
- locate an object.

The command should route to the same underlying application capabilities available through normal UI controls.

### 42.7 Suggested modifications

If AI proposes to create or modify project data, it must produce a proposed Change Set and enter the mutation preview flow.

No project-changing command may skip confirmation merely because the user used imperative language.

### 42.8 Scene card and story assistance

AI may propose Scene Cards, Beats, synopsis text or other story-development structures.

The proposal remains non-authoritative until the user accepts it.

Story Board changes must never silently rewrite screenplay content.

### 42.9 Breakdown assistance

AI may suggest breakdown elements.

Suggested elements remain distinct from confirmed production elements and enter the same acceptance flow as other breakdown suggestions.

### 42.10 Schedule assistance

AI may explain conflicts, identify grouping opportunities and prepare proposed schedule changes.

It must not silently:

- reorder scenes;
- move scenes between shooting days;
- change dates;
- remove scenes;
- or alter schedule assumptions.

### 42.11 Call-sheet assistance

AI may prepare a call-sheet draft from an existing Shooting Day and explain differences between the call sheet and its source schedule.

Creating/updating the project Call Sheet is a mutation and requires confirmation.

### 42.12 Structured rename and replacement

A project-wide rename/replace command must distinguish:

- canonical object names;
- structured references;
- screenplay Character elements;
- production references;
- Story Board references;
- tasks/notes when explicitly selected;
- arbitrary dialogue/action text.

Arbitrary screenplay text is not replaced by default.

### 42.13 Batch operations

AI may prepare batch operations across multiple selected/located objects.

The preview must identify:

- total affected objects;
- affected modules;
- exclusions;
- locked/unauthorized targets;
- conflicts;
- and the exact operation categories.

### 42.14 Permission and state enforcement

AI inherits the current user's effective permissions.

AI must not bypass:

- Viewer/Commenter/Editor/Owner restrictions;
- private-note visibility;
- locked screenplay behavior;
- archived/deleted lifecycle rules;
- approval gates;
- collaboration conflicts.

### 42.15 Change Set and base-version validation

Any project-changing AI request is represented as a proposed Change Set.

The Change Set must retain the base version/state against which it was prepared.

If the project changes before application, the Change Set must be revalidated and may require a new preview/conflict review.

AI must never blindly apply a stale proposal.

### 42.16 Tool boundary

AI does not receive unrestricted write access to project storage.

AI requests application-level tools/operations. Tools validate:

1. target;
2. scope;
3. parameters;
4. user permissions;
5. object/state restrictions;
6. confirmation state.

Only then may a normal application mutation occur.

### 42.17 AI result states

At minimum:

- Informational
- Pending Approval
- Accepted
- Rejected
- Applied
- Stale
- Conflict
- Failed

### 42.18 AI history and activity

AI conversation history may be retained according to product settings.

AI responses do not silently become Project Notes, Story Board content or screenplay content.

Accepted AI mutations become normal OpenFrame history/activity records and remain undoable where supported.

### 42.19 External disclosure

When an external provider is used, the UI must clearly identify the external processing and the project context being transmitted before transmission.

Local AI should not be presented as external/cloud processing.

### 42.20 AI failure safety

If the model, application tool, or external provider fails:

- project content remains unchanged;
- no partial mutation is claimed as complete;
- the user receives a clear failure/retry path.

### 42.21 Cross-module queries

AI may answer questions requiring supported relationship traversal, such as:

- character → scenes → locations;
- scene → breakdown → catalog;
- scene → shots → storyboard;
- scene → shooting day → call sheet;
- draft → changed scenes → affected production planning.

The assistant must preserve source-of-truth distinctions when reporting results.

# 43. Offline and Local-First Behavior

### 43.1 Core offline requirement
Without internet, the user can:

- create/open projects;
- edit Vault;
- edit Story Board;
- write scripts;
- create drafts;
- review/comment locally;
- create breakdowns;
- manage production catalog;
- build shot lists/storyboards;
- schedule;
- create call sheets;
- export supported documents.

### 43.2 Online-only capabilities
Potentially online-only features include:

- external/cloud AI providers;
- software update checks;
- optional external URL metadata retrieval;
- optional local-network collaboration discovery does not require internet.

A configured local AI model may remain available offline.

### 43.3 No “sync pending” dependency
A user opening a project offline must never be blocked by a message saying the project cannot load until synchronized.

### 43.4 Save behavior
The local project is the primary saved state. Save status must refer to local persistence, not cloud state.

### 43.5 External drives
Projects may be stored on ordinary writable local/external drives. The application must warn clearly if the project volume becomes unavailable rather than silently creating a separate inconsistent project copy.


# 44. Saving, Autosave, Recovery and Backup

### 44.1 Autosave
Changes should be saved automatically during normal editing. Users should not need to repeatedly press Save to prevent loss.

### 44.2 Manual Save
A conventional Save command remains available and forces persistence.

### 44.3 Crash recovery
After an abnormal shutdown, reopening the project should offer recovery when unsaved/automatic history exists.

### 44.4 Recovery choices
Show:

- Recover latest autosaved state
- Open last confirmed saved state
- Compare/review differences when possible

### 44.5 Backup
User can create a project backup package at any time.

### 44.6 Backup location
Default backup location is user-selected and can be changed. The user should be able to back up to another disk.

### 44.7 Backup content
A backup must include everything required to restore the project's supported internal content. External linked files should either be copied when the user requests a portable backup or clearly reported as external references.

### 44.8 Backup naming
Include project name, date/time and optional user label.


# 45. Project Portability and Project Packages

### 45.1 Full project export
User can export a portable Project Package for moving the entire project to another machine.

### 45.2 Package intent
A portable project package is different from a review exchange package. It is a full working project transfer/backup.

### 45.3 Import
User chooses Open/Import Project and selects the package.

### 45.4 Collision behavior
If a project with the same identity already exists, the application asks:

- Open as copy
- Replace existing after backup
- Cancel

Default should favor opening as a copy to preserve safety.

### 45.5 Missing external files
The import report lists missing external references. The project itself remains openable.


# 46. Collaboration — Roles and Permissions

### 46.1 Roles
- Owner
- Editor
- Commenter
- Viewer
- Export-only

### 46.2 Owner
Can change project settings, permissions, content, collaboration session ownership, and deletion/restore operations.

### 46.3 Editor
Can edit allowed project content but cannot remove ownership or perform owner-only destructive project operations.

### 46.4 Commenter
Can view permitted content and add/resolve comments where enabled; cannot edit core screenplay/story/production content.

### 46.5 Viewer
Read-only.

### 46.6 Export-only
Can export selected content but cannot alter project data.

### 46.7 Private notes
Private notes remain visible only to their owner. They are excluded from exchange packages and exports unless explicitly chosen, which should be disallowed for ordinary external review packages.


# 47. Remote Collaboration Through Exchange Packages

### 47.1 Purpose
Remote collaboration uses files as the transport layer, not mandatory OpenFrame cloud synchronization.

### 47.2 Package categories
Each major workspace can export a purpose-specific package:

- Story Board package
- Screenplay review package
- Breakdown package
- Shot List package
- Schedule package
- Call Sheet review package

### 47.3 Package content
A package may contain:

- selected page/workspace content;
- source version identity;
- context necessary to understand it;
- comments if selected;
- attachments if selected;
- package metadata.

### 47.4 Export review package
User chooses Share/Exchange → Export Review Package. The UI explains what is included before export.

### 47.5 Receiver import
Receiver chooses Import Review/Exchange. The application identifies the source project/draft/package type.

### 47.6 Review response
Receiver can comment/annotate according to package capabilities and export a response package.

### 47.7 Import response
Owner imports response. Comments/annotations attach to matching scene/text/package objects where safe.

### 47.8 No blind overwrite
Importing a package never silently overwrites the host project.

### 47.9 Stale package
If package source is older than current host content, show source version and current version. Default action is to import as a separate review record or comments-only package.


# 48. Local-Network Real-Time Collaboration

### 48.1 Purpose
Users physically together or on the same private network can optionally collaborate live without an OpenFrame cloud account.

### 48.2 Host session
One user selects Start Collaboration Session. The application displays the session name and join information.

### 48.3 Join
Another OpenFrame desktop joins the session using the displayed local session information.

### 48.4 Shared scope
The host chooses what is shared:

- entire project;
- Story Board;
- Screenplay;
- specific production workspace.

Default should be current project with permissions inherited from session role selection.

### 48.5 Editing
Participants can edit the shared workspace. Changes appear to other participants while connected.

### 48.6 Presence
The UI may show current collaborators and where they are working. Presence is informational.

### 48.7 Disconnect
When a participant disconnects, their last acknowledged changes remain in the shared session. Unacknowledged edits are retained locally where possible and can be exported as a recovery package.

### 48.8 End session
Host can end the collaboration session. The host project remains local and saved.

### 48.9 No permanent hosting requirement
Ending a session does not publish or upload the project to an OpenFrame server.


# 49. Collaboration Conflict Rules

The functional conflict model must prioritize preserving user work.

### 49.1 Different objects
Concurrent edits to different objects are merged without user intervention.

### 49.2 Same card/object, different fields
If one user changes the description and another changes a note, retain both changes where they are independent.

### 49.3 Same text region
If two users edit exactly the same screenplay text region concurrently and changes cannot safely be combined, create a visible conflict state rather than silently choosing one.

### 49.4 Conflict UI
Show:

- version A;
- version B;
- current working copy;
- keep A;
- keep B;
- manually combine.

### 49.5 No silent last-write-wins
The application must not silently discard a collaborator's change merely because another change arrived later.

### 49.6 Offline then reconnect
If a local-network collaborator has gone offline, their later local changes must not automatically overwrite the host. They should be imported/reconciled through the same safe conflict handling used for exchange packages.


# 50. Exchange Package Types and Merge Rules

### 50.1 Story Board package
Contains selected Acts/Sequences/Beats/Scene Cards and comments/attachments selected by the exporter.

On import:

- preview additions/changes;
- preserve host objects by default;
- allow copy-in as a new outline or apply selected changes where mapping is unambiguous.

### 50.2 Screenplay review package
Contains source draft identity, screenplay content snapshot and comments/annotations.

Default import action: attach review comments to the matching draft; do not replace current draft.

### 50.3 Breakdown package
Contains selected scene breakdown and catalog references.

Import can add/update production associations after preview.

### 50.4 Shot List package
Contains selected shots, scene identity and optional storyboard links.

Import updates/adds shot-planning information only after confirmation.

### 50.5 Schedule package
Contains shooting days, scene assignments and day notes.

Import must not delete local schedule days without explicit user action.

### 50.6 Call Sheet review package
Contains call sheet snapshot and review comments. Import adds feedback to the call sheet record; it does not automatically modify the main schedule.


# 51. Undo, Redo, Multi-Select, Drag-and-Drop, Keyboard

### 51.1 Undo/Redo
Undo must work across all major interactive workspaces. The user can undo:

- text edit;
- card create/delete/move;
- board reorder;
- breakdown association;
- shot reorder;
- schedule move;
- note change;
- form edits.

### 51.2 Undo scope
Undo operates within the current editing session and can restore recent saved state changes. Major imported package actions should be treated as a single undoable transaction where practical.

### 51.3 Multi-select
Support multi-select in visual boards and lists.

Use cases:

- move several scene cards;
- export selected scenes;
- apply tag/collection;
- add selected scenes to a production batch.

### 51.4 Drag behavior
While dragging, show insertion target or destination container. Do not require precision beyond the visual card/strip area.

### 51.5 Keyboard
Common shortcuts should include:

- Save
- Undo
- Redo
- Find
- New
- Duplicate where applicable
- Delete
- Navigate previous/next scene
- Switch common screenplay elements

Exact key mapping may follow OS conventions and can be documented separately.


# 52. Delete, Archive, Restore and Destructive Safety

### 52.1 Soft delete
Important creative/production objects move to recoverable deleted state.

### 52.2 Permanent deletion
Permanent deletion requires a second deliberate action and clear warning.

### 52.3 Relationship safety
Deleting a source object does not automatically destroy downstream objects unless the user explicitly confirms the cascade.

Example:

Deleting a Story Board Scene Card does not delete the screenplay scene that was created from it.

### 52.4 Archive vs delete
Archive is preferred for reusable catalog items, projects and people. Delete is reserved for accidental/unwanted records.

### 52.5 Restore
Restore returns object to its previous container/order where possible. If its original container no longer exists, the object is restored to an Unassigned area and the user is notified.


# 53. Script-to-Production Source Rules

### 53.1 Screenplay as production source
Production breakdown is created from a chosen screenplay draft/revision. The source version is displayed at the top of Breakdown.

### 53.2 Production snapshot
When production planning begins, the user can mark a source as the Production Script Version.

### 53.3 Source update
If a newer screenplay revision is approved, user can choose Update Production Source.

### 53.4 Update behavior
The application compares old production source with the new source and reports:

- added scenes;
- removed scenes;
- changed scenes;
- moved scenes;
- changed headings;
- changed page/length information.

### 53.5 Reconciliation
For each affected scene, production data remains attached to stable scene identity when the match is unambiguous. If the scene is new or substantially changed, production review is required.

### 53.6 No destructive auto-update
The application never silently removes props, cast, shots, storyboard panels or location information because the screenplay changed.

### 53.7 Production review queue
Changed scenes can appear in a simple “Needs Review” list.


# 54. Breakdown Reconciliation After Script Changes

### 54.1 Scene deleted
If a scene is removed from the new script, its existing breakdown remains in historical state and is marked “Scene removed from current source.” User may archive it.

### 54.2 Scene added
New scene appears as “Needs Breakdown.”

### 54.3 Heading changed
User is warned that the location/time description changed. Existing production associations remain until reviewed.

### 54.4 Text changes
If the text changes but scene identity remains, existing breakdown stays. Automatic suggestion can identify new likely elements. Existing elements are never silently removed.

### 54.5 Production impact display
Changed scene may show a small impact summary:

- New elements suggested: 3
- Existing elements requiring review: 2
- Shot list exists: Yes — review recommended
- Storyboard exists: Yes — review recommended

The UI should not become an analytics dashboard.


# 55. Shot List and Storyboard Reconciliation

### 55.1 Scene survives unchanged
Visual planning remains unchanged.

### 55.2 Scene text changes
Visual planning remains, with “Scene changed since planning” indicator.

### 55.3 Scene removed
Shot/storyboard records are retained historically and hidden from active production views unless the user chooses to inspect removed content.

### 55.4 Scene duplicated
A duplicated screenplay scene can be treated as a new scene identity. Visual planning is not automatically duplicated unless user chooses Copy Planning.

### 55.5 Scene reordered
Shot/storyboard content follows scene identity, not physical scene number. Display numbering updates automatically.


# 56. Schedule Reconciliation After Script Changes

### 56.1 Scene still exists
Existing schedule assignment remains unless the scene's source mapping becomes ambiguous.

### 56.2 New scene
New scene enters Unscheduled.

### 56.3 Removed scene
Removed scene is marked in schedule history and removed from active future schedule only after user confirmation.

### 56.4 Changed estimated page/time
Schedule shows updated estimate and flags a day if the new estimate affects its total.

### 56.5 Call sheet impact
If a call sheet was prepared from a day whose schedule changed, it is marked “Needs Refresh.” Existing exported PDF is not overwritten.


# 57. Daily Production View

### 57.1 Purpose
A single daily view summarizes a shooting day for the indie team.

### 57.2 Contents
- Date
- Day number
- Scenes
- Locations
- Cast
- Estimated duration
- Key notes
- Call sheet status
- Important breakdown items

### 57.3 Navigation
Click scene → Scene/Breakdown.
Click cast → Cast.
Click location → Location.
Click call sheet → Call Sheet editor.

### 57.4 Edit scope
Daily view allows lightweight day-specific edits but does not bypass the source rules of the underlying modules.


# 58. Basic Production Reports and Sides

### 58.1 Basic reports
The product may provide lightweight reports useful to indie crews:

- scenes by location;
- cast scene list;
- props by scene;
- shooting schedule;
- breakdown summary;
- shot list.

### 58.2 Sides
P1 lightweight sides feature:

User selects shooting day → Generate Sides.

The application creates a document containing only the relevant screenplay scenes from that day, with optional call sheet cover information.

### 58.3 Sides source
Sides are snapshots from the selected screenplay source. Editing a side document outside the app does not change the screenplay.

### 58.4 Exports
PDF required.


# 59. Templates

### 59.1 Purpose
Templates accelerate repeated indie workflows without adding mandatory setup.

### 59.2 Template types
- Project starter
- Moodboard starter
- Story Board starter
- Call sheet template
- Shot list template
- Breakdown document template
- Simple budget template

### 59.3 User-created templates
Users can save a configured document/workspace as a template.

### 59.4 Template isolation
Changing a template does not change existing projects created from it.

### 59.5 No marketplace requirement
A template marketplace is outside core scope.


# 60. Export and Printing

### 60.1 Export principle
Every major useful workspace must have an obvious Export action.

### 60.2 Supported output
Screenplay: PDF/FDX/Fountain/DOCX.

Outline/Story Board: PDF.

Moodboard: PDF/image presentation.

Breakdown: PDF/report.

Shot List: PDF.

Storyboard: PDF/image sheet.

Schedule/Stripboard: PDF.

Call Sheet: PDF.

Sides: PDF.

Project: portable project package.

Review: exchange package.

### 60.3 Selection
Where practical, export supports:

- current item;
- selected items;
- current section;
- entire project document.

### 60.4 Print
Print uses the same document-generation logic as PDF so printed output remains predictable.


# 61. Empty States

Every empty workspace needs a purpose-oriented message and one or two actions.

### 61.1 Idea Vault
> “This is where you throw anything about the film. Add a note, image, link, file, or voice note.”

Primary: Add.

### 61.2 Story Board
> “Start shaping the story. Add an Act, Sequence, Beat, or Scene Card.”

Primary: Add Scene.

### 61.3 Screenplay
> “Write your screenplay or import one you already have.”

Primary: New Screenplay / Import.

### 61.4 Breakdown
> “Choose a screenplay draft to begin breaking down scenes.”

Primary: Select Source.

### 61.5 Schedule
> “Create a shooting schedule from your screenplay.”

Primary: Create Schedule.

### 61.6 Call Sheets
> “Call sheets will appear here after you create shooting days.”

Primary: Open Schedule.


# 62. Error and Recovery Behavior

### 62.1 Error principles
Errors must:

- explain what failed in plain language;
- preserve user work where possible;
- provide next action;
- avoid exposing meaningless internal jargon.

### 62.2 Save failure
Message:

> “OpenFrame could not save the latest changes to the project.”

Actions:

- Retry Save
- Save Backup Copy
- Keep Editing

### 62.3 Import failure
Show:

- source file;
- failure stage;
- whether a partial preview is available;
- whether the source file remains untouched.

### 62.4 Export failure
Source project remains unchanged. User can retry to another destination.

### 62.5 Missing linked file
Show a relink action and project path information without deleting the reference.

### 62.6 Collaboration disconnect
Show that live session ended, then return the user to local project state.


# 63. Source-of-Truth and Connection Matrix

The following matrix is mandatory product behavior.

| From | To | Relationship | Automatic? | User control |
|---|---|---|---|---|
| Global Idea Vault | Project Idea Vault | Copy | No | User initiates |
| Project Idea Vault | Story | Copy/convert | No | User initiates |
| Story Board | Screenplay | Build/copy structure | No | User initiates |
| Screenplay | Breakdown | Source snapshot/referenced source | User initiates | Yes |
| Screenplay | Shot List | Scene-based creation | User initiates | Yes |
| Screenplay | Storyboard | Scene-based creation | User initiates | Yes |
| Breakdown | Catalog | Reuse/create production identity | Suggest/confirm | Yes |
| Catalog | Schedule | Scene/day requirements | Derived | Yes |
| Schedule | Call Sheet | Document prefill | Generated by user | Yes |
| Storyboard | Shot List | Optional reference reuse | User initiates | Yes |
| Shot List | Call Sheet | Optional reference/notes | User initiates | Yes |

### 63.1 Connections explicitly not required
The product does not require:

- Idea Vault live sync with screenplay;
- Story Board live sync with every screenplay edit;
- Call Sheet edits to rewrite schedule;
- Moodboard edits to modify scene production data;
- shot changes to rewrite screenplay;
- production catalog edits to rewrite script prose;
- AI suggestions to become data automatically.


# 64. End-to-End Functional Walkthrough — First-Time Filmmaker

This is the canonical workflow QA should be able to execute.

### Step 1 — Create project
User creates `BLACK RAIN` as Feature Film.

Expected: Project Home opens.

### Step 2 — Capture idea
User adds text note, image, URL, PDF, voice note.

Expected: all items appear in Project Idea Vault without mandatory classification.

### Step 3 — Build outline
User creates Act 1, Sequence `Hero Introduction`, several Scene Cards.

Expected: cards are small rectangles, no manual numbers.

### Step 4 — Reorder
User drags Scene Card 5 before Scene Card 3.

Expected: order changes and is undoable.

### Step 5 — Write
User gives cards valid scene headings and clicks Build Screenplay.

Expected: screenplay created in card order with scene numbers generated by screenplay order.

### Step 6 — Draft
User writes script and creates Draft 2.

Expected: Draft 1 remains unchanged.

### Step 7 — Review
Reviewer adds comments to selected text.

Expected: comments are retained and can be resolved.

### Step 8 — Lock
User locks Draft 6.

Expected: editing requires Start Revision.

### Step 9 — Breakdown
User chooses Locked Draft as production source.

Expected: scene list populated; user can manually tag and confirm suggestions.

### Step 10 — Catalog
Confirmed props/locations/cast become reusable production catalog items.

### Step 11 — Visual planning
User creates shot list and storyboard for selected scenes.

### Step 12 — Schedule
User creates shooting days and drags scene strips onto days.

### Step 13 — Conflict
User schedules same actor in incompatible locations.

Expected: advisory conflict warning.

### Step 14 — Call sheet
User generates call sheet from Shoot Day 3.

Expected: scenes, location and cast details prefilled.

### Step 15 — Export
User exports screenplay PDF, shot list PDF, schedule PDF and call sheet PDF.

Expected: snapshots created; project remains unchanged.


# 65. End-to-End Functional Walkthrough — Existing Screenplay User

A user who already has a script should not be forced through Idea Vault or Story Board.

1. Create/open project.
2. Import PDF/FDX/Fountain/TXT/DOCX/paste.
3. Review import preview.
4. Import as new screenplay/draft.
5. Open Breakdown.
6. Confirm/suggest breakdown elements.
7. Build Catalog.
8. Add Locations/Cast/Crew.
9. Create Shot List/Storyboard.
10. Create Schedule.
11. Generate Call Sheets.

The Story Board remains optional for reference.


# 66. End-to-End Functional Walkthrough — Short Film

A short film may use only:

Idea Vault → Story Board → Screenplay → Breakdown → Shot List → Schedule → Call Sheet.

The application must not force the user to fill:

- budget;
- character profiles;
- moodboards;
- storyboard;
- complex schedule metadata;
- episodic structures.

Unused modules remain accessible but visually quiet.


# 67. End-to-End Functional Walkthrough — Episodic

1. Create Series.
2. Create Season.
3. Create episodes.
4. Develop each episode on its own Story Board.
5. Write each episode screenplay.
6. Maintain series-level recurring character/location references.
7. Break down selected episode scripts.
8. Schedule production as required.

Season-level navigation remains simple and episode-centric. No giant season-room database is required.


# 68. Functional Acceptance Matrix — Core Creative

| ID | Requirement | Priority | Verification |
|---|---|---|---|
| FSD-IDEA-001 | User can create a text Idea Vault item without metadata. | P0 | Create note and save. |
| FSD-IDEA-002 | User can add image, URL, PDF, document, audio, video, screenshot, sketch and ordinary file items. | P0 | One test per item type. |
| FSD-IDEA-003 | Global and project Idea Vaults exist independently. | P0 | Create global item and copy to project; edit copy. |
| FSD-IDEA-004 | Move to Story creates a copy and leaves original Vault item intact. | P0 | Convert item and edit destination. |
| FSD-STORY-001 | Acts can contain sequences and/or scenes. | P0 | Create and move objects. |
| FSD-STORY-002 | Sequence is a simple named scene container. | P0 | Create sequence with one name field. |
| FSD-STORY-003 | Scene cards are compact and show short description without manual numbering. | P0 | Create/reorder 20 cards and inspect UI. |
| FSD-STORY-004 | Dragging cards changes story order and is undoable. | P0 | Drag then undo. |
| FSD-STORY-005 | Parking Lot preserves unused creative cards. | P0 | Move card, close/reopen project, restore. |
| FSD-SCRIPT-001 | Screenplay supports required screenplay element types. | P0 | Write sample script. |
| FSD-SCRIPT-002 | Build Screenplay creates scenes from selected Story Board cards. | P0 | Build and verify order/headings. |
| FSD-SCRIPT-003 | Idea Vault is not live synchronized with screenplay. | P0 | Edit source note and verify script unchanged. |
| FSD-SCRIPT-004 | Named drafts preserve prior versions. | P0 | Create multiple drafts and compare. |
| FSD-SCRIPT-005 | Script comments can be created and resolved. | P0 | Add comment, reply, resolve. |
| FSD-SCRIPT-006 | Locked script cannot be casually edited without starting a revision. | P0 | Lock and attempt edit. |


# 69. Functional Acceptance Matrix — Production

| ID | Requirement | Priority | Verification |
|---|---|---|---|
| FSD-BRK-001 | Breakdown can be created from a selected screenplay version. | P0 | Select draft and open breakdown. |
| FSD-BRK-002 | Manual tagging creates scene-specific breakdown associations. | P0 | Tag prop/location/cast. |
| FSD-BRK-003 | Automatic breakdown suggestions require confirmation. | P0 | Accept/reject/edit suggestion. |
| FSD-CAT-001 | Confirmed elements can be reused as catalog items. | P0 | Use same prop across two scenes. |
| FSD-LOC-001 | Locations display scene usage. | P1 | Create location and inspect usage. |
| FSD-CAST-001 | Cast records connect actors to characters and scenes. | P1 | Assign actor and inspect scene list. |
| FSD-SHOT-001 | Shots are ordered by drag/drop and tied to scenes. | P0 | Create/reorder shots. |
| FSD-STB-001 | Storyboard panels can be associated with shots/scenes. | P1 | Create panel and attach to shot. |
| FSD-SCH-001 | Scenes enter Unscheduled when a schedule is created. | P0 | Create schedule and inspect. |
| FSD-SCH-002 | Scenes can be dragged into shooting days. | P0 | Drag and verify day. |
| FSD-SCH-003 | Basic conflicts are displayed without automatic forced correction. | P1 | Create cast/location conflict. |
| FSD-CALL-001 | Call sheet can be generated from shooting day data. | P0 | Create day and call sheet. |
| FSD-CALL-002 | Call sheet changes do not silently rewrite schedule. | P0 | Edit call sheet and inspect schedule. |


# 70. Functional Acceptance Matrix — Offline and Collaboration

| ID | Requirement | Priority | Verification |
|---|---|---|---|
| FSD-OFF-001 | Core workflows are available with network disconnected. | P0 | Disable network and exercise core flows. |
| FSD-OFF-002 | Local save remains source of truth. | P0 | Edit/save/reopen offline. |
| FSD-COL-001 | Project can export a portable exchange package. | P0 | Export and inspect package in another installation. |
| FSD-COL-002 | Review package import does not blindly overwrite host project. | P0 | Import stale response package. |
| FSD-COL-003 | Local-network session allows shared work without OpenFrame cloud. | P1 | Host and join on same LAN. |
| FSD-COL-004 | Same-object conflicts are surfaced rather than silently lost. | P1 | Concurrent conflicting edit test. |
| FSD-AI-001 | AI is optional and unavailable core functions still work. | P1 | Run with AI disabled. |
| FSD-AI-002 | AI modifications require user confirmation. | P1 | Ask AI to create card; verify confirmation. |


# 71. Data Mutation Rules by Module

The following rules define whether an action is additive, mutating, destructive, or snapshot-based.

| Action | Type | Default safety behavior |
|---|---|---|
| Add Idea | Additive | Immediate save |
| Move Idea to Story | Copy | Original retained |
| Move Scene Card | Mutation | Undo available |
| Duplicate Scene Card | Create new object | Original unchanged |
| Build Screenplay | Create draft | Existing drafts untouched |
| Edit Draft | Mutation to current draft | Auto history retained |
| Create Named Draft | Copy/version | Parent retained |
| Lock Draft | State change | Explicit confirmation |
| Start Revision | Create new revision | Locked source retained |
| Add Breakdown Element | Additive/link | Undo available |
| Remove Breakdown Element | Association removal | Catalog item retained |
| Delete Catalog Item | State change/archive preferred | Scene associations protected |
| Schedule Scene | Assignment | Undo available |
| Edit Call Sheet | Document mutation | Schedule unchanged |
| Export | Snapshot | Project unchanged |
| Import Exchange | Add/update after preview | No blind overwrite |


# 72. State and Status Rules

### Project
`Active / Archived / Closed`

### Idea Vault Item
`Active / Pinned / Deleted`

### Story Card
`Active / Parking Lot / Deleted`

### Draft
`Current / Named / Locked / Superseded / Deleted`

### Review Comment
`Open / In Discussion / Resolved`

### Breakdown Element
`Suggested / Confirmed / Rejected / Removed`

### Catalog Item
`Required / Searching / Shortlisted / Confirmed / Not Required / Archived`

### Location
`Idea / Shortlisted / Confirmed / Rejected`

### Shooting Day
`Planned / Ready / Completed / Archived`

### Call Sheet
`Draft / Final / Superseded / Archived`

Statuses must be human-readable and not expose internal implementation states to ordinary users.


# 73. Cross-Module Traceability

The following traceability rules are required so QA can verify the application as one product rather than isolated modules.

### Idea → Story
A copied Vault item can become a Story object. The original remains intact.

### Story → Screenplay
A selected sequence of Scene Cards can create screenplay scenes. Card order drives creation order. Scene numbers are derived later.

### Screenplay → Breakdown
A chosen draft/revision becomes production source. Scene identities allow production data to persist through revisions where mapping is clear.

### Breakdown → Catalog
Confirmed items are reusable across scenes.

### Screenplay → Shot/Storyboard
Visual planning can start from screenplay scenes without writing camera directions into the script.

### Catalog + Screenplay → Schedule
Scheduling displays scenes plus required locations/cast and other practical data.

### Schedule → Call Sheet
Call sheet is prefilled from schedule and production data but remains a snapshot document.

### Export → External collaboration
Any exported document/package is a snapshot. Re-import behavior is explicitly defined by the package/document type.


# 74. Feature Access by Project Type

| Feature | Feature | Short | Episodic |
|---|---|---|---|
| Idea Vault | Yes | Yes | Yes |
| Story Board | Yes | Yes | Yes |
| Acts | Yes | Optional | Yes |
| Sequences | Yes | Optional | Yes |
| Beats | Yes | Optional | Yes |
| Screenplay | Yes | Yes | Yes |
| Drafts/Versions | Yes | Yes | Yes |
| Breakdown | Yes | Optional | Yes |
| Shot List | Yes | Yes | Yes |
| Storyboard | Optional | Optional | Optional |
| Schedule | Yes | Optional | Yes |
| Call Sheets | Yes | Optional | Yes |
| Budget Snapshot | Optional | Optional | Optional |
| Season/Episode | No | No | Yes |

“Optional” means the feature is available but must not be forced into the primary workflow.


# 75. UX-Level Functional Requirements for Simplicity

### 75.1 One obvious primary action
Every major workspace must have one obvious primary action:

Idea Vault → Add

Story Board → Add Scene / Add Sequence

Screenplay → Write / Import

Breakdown → Tag / Suggest

Production → Choose workspace

Schedule → Create Shooting Day

Call Sheet → Create / Update

### 75.2 Progressive disclosure
Advanced fields are hidden behind Details/Edit/More. Default forms remain small.

### 75.3 No terminology gate
The user can accomplish normal actions without understanding professional film terminology beyond what is necessary for the document being created.

### 75.4 No forced workflow
Users can import an existing screenplay and bypass Idea Vault/Story Board. Users can also develop for months without creating a schedule.

### 75.5 Visual density
The Story Board and Stripboard may be information-dense, but each card/strip remains visually scannable. Avoid spreadsheet-style walls of metadata.


# 76. QA Test Design Guidance

QA should test OpenFrame in four layers.

### Layer 1 — Feature behavior
Does each feature perform its stated actions?

### Layer 2 — State transitions
Does the object move to the correct state after each action?

### Layer 3 — Cross-module behavior
Does the expected data/identity connection survive between modules?

### Layer 4 — Recovery/safety
Does undo, delete/restore, import/export, backup and conflict handling preserve work?

Every P0 requirement must have at least one positive test and one destructive/error-path test.

Examples:

**Scene Card:** create, reorder, duplicate, delete, restore, build screenplay.

**Draft:** create, edit, compare, export, lock, revision.

**Breakdown:** suggest, accept, reject, remove, script-change reconciliation.

**Schedule:** drag, move, conflict, date change, call-sheet refresh.

**Exchange:** export, import matching, import stale, conflict, cancel.


# 77. Regression-Critical Workflows

The following workflows are regression-critical because downstream modules depend on them.

1. Create Scene Card → Build Screenplay → Open Breakdown.
2. Import FDX → Breakdown → Schedule → Call Sheet.
3. Draft 1 → Draft 2 → Compare → Lock → Revision A.
4. Breakdown Element → Catalog → Schedule.
5. Storyboard → Shot List → Export.
6. Schedule change → Call Sheet marked stale → Refresh.
7. Offline edit → Save → Close → Reopen.
8. Exchange export → second machine import → response package → host import.
9. Local network collaboration → disconnect → project remains intact.
10. Delete/restore scene with linked downstream data.


# 78. Product-Behavior Edge Cases

### 78.1 Duplicate scene card with no heading
Allowed in Story Board. Build Screenplay later requires a valid heading.

### 78.2 Empty sequence
Allowed. It may remain empty.

### 78.3 Empty Act
Allowed.

### 78.4 Scene card with very long description
Store full text. Truncate only visual card display.

### 78.5 Same scene description twice
Allowed. Scene identity is independent of text similarity.

### 78.6 Same character name twice
Before creating a second character, show likely match but allow the user to create another if they genuinely need two characters with similar names.

### 78.7 Same prop name in multiple scenes
Default to reuse existing catalog item when user confirms.

### 78.8 Deleted sequence containing cards
Offer move children rather than automatic loss.

### 78.9 Screenplay scene reordered independently from board
Supported. Board is a reference layer unless the user deliberately applies an order change.

### 78.10 Script imported into a project with existing Story Board
Import creates a screenplay/draft without modifying Story Board.

### 78.11 Schedule created from screenplay with no breakdown
Allowed. Breakdown is helpful but not required to schedule.

### 78.12 Call sheet generated with incomplete cast call times
Allowed with visible missing-value indication; user can fill it manually.

### 78.13 AI unavailable
Hide/disable AI features; core app continues normally.


# 79. Product Boundaries That Engineering Must Not Cross

The following are not implied by this FSD and must not be implemented as hidden “future-proofing” features that complicate the user experience:

- no enterprise accounting workflows;
- no payroll engine;
- no union rules engine;
- no VFX shot tracking system;
- no post-production asset review platform;
- no distribution CRM;
- no talent marketplace;
- no large-scale studio resource planner;
- no mandatory cloud synchronization;
- no required user classification for every Idea Vault item;
- no complex story branching engine;
- no mandatory character arc scoring;
- no screenplay quality scoring dashboard;
- no automatic schedule optimizer that silently moves scenes;
- no automatic AI rewriting of screenplay text;
- no giant metadata form on Scene Cards;
- no requirement to maintain scene numbers manually on the Story Board.


# 80. Implementation Handoff Checklist

Before engineering considers a module implementation-ready, the team should be able to answer all of the following without guessing:

- What opens this feature?
- What is the default screen state?
- What is the minimum required input?
- What fields are optional?
- What does Create do?
- What does Edit do?
- What does Duplicate do?
- What does Delete do?
- What does Undo restore?
- What does Save persist?
- What happens if the user cancels?
- What happens if a required object is missing?
- What happens if the object is already used elsewhere?
- What happens if the project is offline?
- What happens if the user exports it?
- What happens if another user imports/edits it?
- What downstream objects are affected?
- What downstream objects are deliberately not affected?
- What is authoritative after the action?
- What is visible in history?
- How can QA verify it?

If any answer is not defined by this FSD or an explicitly linked companion specification, it should be logged as a specification question before implementation rather than silently invented in code.


# 81. Requirement ID Convention

PRD requirement IDs use:

`PRD-[MODULE]-NNN`

Feature requirement IDs use:

`FSD-[MODULE]-NNN`

Examples:

- FSD-IDEA-001
- FSD-STORY-001
- FSD-SCRIPT-001
- FSD-BRK-001
- FSD-SCH-001
- FSD-CALL-001
- FSD-COL-001
- FSD-AI-001

QA test IDs should use:

`TEST-[MODULE]-NNN`

UX screen IDs may use:

`UI-[MODULE]-NNN`

These identifiers allow the PRD/FSD, UX specification and QA test specification to reference the same product behavior without relying on page names alone.


# 82. Traceability to the Source PRD

The FSD is derived from the source PRD and preserves the core product decisions described there: local-first desktop operation, indie target users, Idea Vault, simple Story Board, screenplay, breakdown, pre-production, schedule, call sheets, exchange packages, local-network collaboration, optional AI, progressive disclosure, no duplicate entry, and explicit scope boundaries. The source PRD establishes the high-level information hierarchy and product boundaries; this FSD expands those sections into user actions, states, mutation rules, connections and acceptance behavior. fileciteturn2file0L4-L23

The source PRD's core chain is:

`IDEA VAULT → VISUAL STORY BOARD → ACTS / SEQUENCES / BEATS / SCENE CARDS → SCREENPLAY → REVIEW / REWRITE / VERSION → LOCKED SHOOTING SCRIPT → SCRIPT BREAKDOWN → CAST / LOCATIONS / PROPS / WARDROBE / OTHER ELEMENTS → SHOT LIST / STORYBOARD / MOODBOARD → STRIPBOARD / SHOOTING SCHEDULE → CALL SHEETS` and this FSD treats that chain as the principal cross-module behavior. fileciteturn2file0L27-L67


## Requirement-Level Traceability Rules

> **A project specification is not considered functionally complete while any PRD requirement has no FSD mapping.**

> **A feature appearing in the FSD but lacking a PRD requirement ID is considered unapproved scope.**

## FSD-to-PRD contradiction test

Before implementation, QA/product must verify:

```text
Every FSD requirement:
    ↓
Has PRD Requirement ID
    ↓
Has same priority
    ↓
Doesn't violate scope boundary
    ↓
Has functional behavior
    ↓
Has acceptance test
```

## Two-Document Contract

> The PRD defines product intent, scope, priority and product-level decisions.

> The FSD defines the observable behavior required to satisfy the PRD. It may clarify behavior but may not change product intent or scope.

> When the documents conflict, the conflict must be resolved explicitly before implementation; engineering must not select whichever interpretation is more convenient.


# 83. Final Functional Definition

OpenFrame Studio is functionally complete when an independent filmmaker can:

1. Create or import a project.
2. Throw any film-related material into a Global or Project Idea Vault without categorizing it.
3. Create an Act/Sequence/Beat/Scene Card outline through an easy drag-and-drop Story Board.
4. Keep the Story Board as a visual reference even after writing begins.
5. Build a screenplay from selected Scene Cards or write/import a screenplay directly.
6. Work on multiple named drafts with automatic history.
7. Compare drafts and conduct lightweight review rounds.
8. Lock a shooting draft and create controlled post-lock revisions.
9. Break down the screenplay manually and with confirmed automatic suggestions.
10. Maintain simple production catalogs for locations, cast, props and other breakdown elements.
11. Create moodboards, storyboards and shot lists without requiring all of them.
12. Build a stripboard by dragging screenplay scenes into shooting days.
13. Receive practical schedule conflict warnings and advisory grouping suggestions.
14. Generate practical call sheets from shooting days.
15. Work offline from local project files.
16. Move projects between machines using portable project packages.
17. Collaborate remotely using portable review/exchange packages instead of mandatory cloud sync.
18. Collaborate live on a private local network when desired.
19. Use AI as an optional assistant without surrendering control of the project.
20. Export professional working documents without losing ownership of the underlying project.

The product is successful functionally when the filmmaker can perform the full journey with minimal duplicate entry, without a mandatory workflow wizard, and without being forced to understand or configure production-management concepts that are irrelevant to the current stage of their film.


# 84. Final Non-Functional-to-Functional Boundary

This FSD intentionally stops at user-visible product behavior. The engineering team must define separately, in the ESD and supporting technical specifications, implementation decisions such as:

- programming language;
- desktop framework;
- storage engine;
- project file internals;
- concurrency algorithm;
- network transport;
- encryption method;
- AI provider implementation;
- parsing libraries;
- PDF/FDX/DOCX generation libraries;
- update mechanism;
- telemetry implementation;
- logging architecture.

Those choices are valid engineering decisions only when they satisfy this FSD's functional behavior. An implementation choice must not be allowed to simplify engineering at the cost of violating the product behavior described here.


# 85. Engineering Sign-Off Gate

A functional area is ready for implementation when:

- its screens/actions are covered by a UX specification;
- its user-visible behavior is covered by this FSD;
- its object relationships are covered by the domain/data specification;
- its offline/collaboration behavior is covered by the collaboration specification where relevant;
- its import/export behavior is covered where relevant;
- its acceptance criteria have IDs;
- its unresolved questions are zero or explicitly recorded as future-scope decisions.

A feature is not considered “done” merely because its UI exists. It is done only when its behavior, cross-module relationship, error handling, persistence, undo/recovery behavior, export behavior where applicable, and acceptance tests all pass.


# Appendix A — Requirement-Level Traceability Ledger

The following ledger maps PRD requirement IDs to the functional FSD requirements that implement them. It is intended to be a completeness checklist rather than a copy of the source document.

| PRD Requirement ID | FSD Requirement ID(s) |
|---|---|
|PRD-CORE-001|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-IDEA-001|FSD-IDEA-001..FSD-IDEA-002, FSD-IDEA-014|
|PRD-IDEA-002|FSD-IDEA-003..FSD-IDEA-004, FSD-IDEA-011..FSD-IDEA-013|
|PRD-IDEA-003|FSD-IDEA-005..FSD-IDEA-008, FSD-IDEA-019|
|PRD-IDEA-004|FSD-IDEA-009..FSD-IDEA-010, FSD-IDEA-020|
|PRD-IDEA-005|FSD-IDEA-015..FSD-IDEA-017|
|PRD-STORY-001|FSD-STORY-001..FSD-STORY-003, FSD-STORY-017..FSD-STORY-019, FSD-STORY-028|
|PRD-STORY-002|FSD-STORY-004, FSD-STORY-029..FSD-STORY-030|
|PRD-STORY-003|FSD-STORY-005..FSD-STORY-007, FSD-STORY-026..FSD-STORY-027|
|PRD-STORY-004|FSD-STORY-008..FSD-STORY-016|
|PRD-STORY-005|FSD-STORY-020..FSD-STORY-025|
|PRD-STORY-006|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-STORY-007|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-SCRIPT-001|FSD-SCRIPT-001..FSD-SCRIPT-022|
|PRD-SCRIPT-002|FSD-SCRIPT-023..FSD-SCRIPT-027|
|PRD-SCRIPT-003|FSD-SCRIPT-028..FSD-SCRIPT-033|
|PRD-SCRIPT-004|FSD-SCRIPT-034..FSD-SCRIPT-037|
|PRD-SCRIPT-005|FSD-SCRIPT-038..FSD-SCRIPT-044|
|PRD-SCRIPT-006|FSD-SCRIPT-045..FSD-SCRIPT-050|
|PRD-SCRIPT-007|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-EP-001|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-EP-002|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-BRK-001|FSD-BREAKDOWN-001..FSD-BREAKDOWN-019|
|PRD-PROD-001|FSD-PROD-001..FSD-PROD-003|
|PRD-LOC-001|FSD-PROD-004..FSD-PROD-007|
|PRD-CAST-001|FSD-PROD-008..FSD-PROD-010|
|PRD-VIS-001|FSD-PROD-011..FSD-PROD-013|
|PRD-STB-001|FSD-PROD-014..FSD-PROD-017|
|PRD-STB-002|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-STB-003|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-SHOT-001|FSD-PROD-018..FSD-PROD-021|
|PRD-SCHED-001|FSD-SCHED-001..FSD-SCHED-009, FSD-SCHED-014..FSD-SCHED-016|
|PRD-SCHED-002|FSD-SCHED-010..FSD-SCHED-013|
|PRD-SCHED-003|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-CALL-001|FSD-CALL-001..FSD-CALL-012|
|PRD-COL-001|FSD-COL-001..FSD-COL-005|
|PRD-COL-002|FSD-COL-007|
|PRD-COL-003|FSD-COL-006, FSD-COL-008..FSD-COL-011|
|PRD-COL-004|FSD-COL-012..FSD-COL-016|
|PRD-COL-005|FSD-COL-017..FSD-COL-019, FSD-COL-021..FSD-COL-022|
|PRD-COL-006|FSD-COL-020|
|PRD-OFF-001|FSD-OFF-001..FSD-OFF-010|
|PRD-AI-001|FSD-AI-001..FSD-AI-027|
|PRD-AI-002|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-PROD-002|FSD-PROD-022|
|PRD-PROD-003|FSD-PROD-023|
|PRD-PROD-004|FSD-PROD-024|
|PRD-PROD-005|FSD-PROD-025..FSD-PROD-026|
|PRD-PROD-006|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-PROD-007|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-TPL-001|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-UX-001|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-CORE-002|FSD-IDEA-018|
|PRD-CORE-003|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|
|PRD-CORE-004|Covered by functional sections and cross-cutting constraints; no standalone inventory ID.|

# Appendix B — Core Object Behavioral Summary

| Object | Create | Edit | Duplicate | Move | Delete | Export | Downstream use |
|---|---|---|---|---|---|---|---|
| Idea Vault Item | Yes | Yes | Yes/copy | Folder/collection | Soft delete | Package/file | Optional Story copy |
| Act | Yes | Yes | Optional | Yes | Safe delete | Outline export | Story structure |
| Sequence | Yes | Yes | Optional | Yes | Safe delete | Outline export | Story structure |
| Beat | Yes | Yes | Yes | Yes | Soft delete | Outline export | Optional Scene creation |
| Scene Card | Yes | Yes | Yes | Yes | Soft delete | Outline export | Optional Screenplay creation |
| Character | Yes | Yes | Optional | N/A | Archive | Report | Screenplay/Production |
| Draft | Yes | Yes | Yes/version | N/A | Soft delete | Script formats | Breakdown/review |
| Breakdown Element | Yes | Yes | Catalog reuse | Scene association | Remove association | Report | Catalog/Schedule |
| Catalog Item | Yes | Yes | Optional | N/A | Archive | Report | Schedule/Call Sheet |
| Shot | Yes | Yes | Yes | Yes | Soft delete | Shot list | Storyboard/Call Sheet |
| Storyboard Panel | Yes | Yes | Yes | Yes | Soft delete | Board export | Shot List |
| Shooting Day | Yes | Yes | Duplicate | Calendar/schedule | Archive | Schedule PDF | Call Sheet |
| Call Sheet | Yes | Yes | Duplicate | N/A | Archive | PDF | External distribution |


# Appendix C — Default UI Labels

For consistency, the product should prefer these labels unless a UX specification defines a justified alternative:

- New Project
- Open Project
- Idea Vault
- Add Note
- Add File
- Add Image
- Add URL
- Add Voice Note
- Story Board
- Add Act
- Add Sequence
- Add Beat
- Add Scene
- Parking Lot
- Build Screenplay
- Screenplay
- New Draft
- Compare Drafts
- Start Review
- Comment
- Resolve
- Lock as Shooting Draft
- Start Revision
- Breakdown
- Suggest Elements
- Confirm
- Reject
- Edit Suggestion
- Production
- Locations
- Cast & Crew
- Catalog
- Moodboards
- Storyboards
- Shot List
- Schedule
- Unscheduled
- Create Shooting Day
- Call Sheets
- Create Call Sheet
- Export
- Import
- Share / Exchange
- Start Collaboration Session
- Search
- Undo
- Redo


# Appendix D — Definition of Done for the Functional Layer

The functional layer of OpenFrame Studio is considered specified when:

- every P0 capability in the source PRD has explicit user behavior;
- every P1 capability has explicit behavior or is intentionally described as a lightweight extension;
- P2 capabilities have a bounded functional definition if retained in scope;
- each major object has lifecycle rules;
- cross-module source-of-truth rules are explicit;
- offline behavior is explicit;
- collaboration behavior is explicit;
- import/export behavior is explicit;
- destructive actions have safe behavior;
- undo/redo expectations exist;
- acceptance criteria exist for core workflows;
- no requirement relies on an unstated cloud backend;
- no requirement assumes an enterprise production environment;
- no requirement requires a user to re-enter information purely because another module needs it;
- no automatic process can silently overwrite creative or production decisions.

This document should be treated as the functional baseline. Changes to it after engineering begins should be versioned and reviewed like product changes, especially changes to source-of-truth rules, object identity, import/export behavior, or collaboration behavior.

---

*Document generation note:** This FSD was generated from the uploaded OpenFrame Studio Mega PRD and the explicit product decisions supplied in the current conversation. Approximate size: 17,853 words / 3,148 lines.

# 86. Screen-by-Screen Functional Contract

This section defines the expected functional composition of each primary screen. Visual styling, typography and exact pixel dimensions belong in the UX/UI specification, but the functional regions and available actions below are mandatory.

## 86.1 Home

**Purpose:** project discovery and continuation.

**Functional regions:**

1. Recent Projects list.
2. New Project button.
3. Open Project button.
4. Global Idea Vault shortcut.
5. Archived Projects access.
6. Search/command access.

**Recent project row behavior:** selecting a row opens that project. Context actions include Rename, Duplicate, Archive and Reveal in File Manager where applicable.

**No-data state:** show a simple explanation and New Project/Open Project actions.

## 86.2 Project Home

**Functional regions:** project identity, Continue, Quick Access, Recent, Project Files and lightweight status.

**Continue action:** opens the most recently active meaningful workspace. If that workspace no longer exists, fallback is Project Home.

## 86.3 Idea Vault

**Functional regions:** add, search, view selector, folder/collection navigation, item canvas/list, item preview/editor.

The Add control must never be hidden behind a settings menu.

## 86.4 Story Board

**Functional regions:** act/sequence hierarchy, cards, Parking Lot, view selector, undo/redo, build-screenplay action.

Only story objects relevant to current hierarchy appear on the primary canvas. Detailed metadata opens only when the card is expanded.

## 86.5 Screenplay

**Functional regions:** scene navigator, screenplay page, element selector, comments/notes toggle, draft selector, export, search.

Focus Mode removes nonessential chrome without changing functionality.

## 86.6 Breakdown

**Functional regions:** scene list, screenplay text, breakdown panel, category groups, suggested-elements area, catalog selector.

## 86.7 Production

Production is a grouped workspace. It contains tabs/sections for Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Schedule and lightweight Budget.

The Production landing view is a gateway; it does not show every detail at once.

## 86.8 Call Sheets

**Functional regions:** call-sheet list, Create Call Sheet, status filter, document editor, export, refresh-from-schedule action.


# 87. Form and Field Behavior Rules

Every form in OpenFrame follows the same functional principles.

### 87.1 Required fields
Required fields are limited to information necessary to create a valid object. A required field is visibly marked. The user receives an inline message if they try to commit it empty.

### 87.2 Optional fields
Optional fields are collapsed or omitted from the default form until requested.

### 87.3 Save model
Short card/form edits save automatically. Longer dialogs may have Save and Cancel. Cancel must discard only edits made in that dialog since it opened.

### 87.4 Escape
Escape closes a transient panel/dialog if there are no unsaved edits; if edits exist, it follows the same discard confirmation as Cancel.

### 87.5 Invalid input
Invalid input remains visible in the same context. The application should not navigate the user away merely to report an error.

### 87.6 Duplicate names
Duplicate names are allowed unless the object type requires uniqueness for functional reasons. Where duplicate names are allowed, context such as parent sequence/location/project makes the distinction.

### 87.7 Long text
Long text is never silently truncated. UI controls may scroll or collapse visually while storing the complete content.

### 87.8 Paste
Text paste must preserve readable content. Screenplay paste follows the screenplay import/parser flow rather than becoming arbitrary rich text.


# 88. Idea Vault — Granular Functional Contract

### 88.1 Quick-add menu
`Add` opens a compact menu with Note, Image, URL, File, Audio/Voice Note and Video. Recent commonly used choices can appear first but the complete set remains available.

### 88.2 Drag-in behavior
Dragging files into the visible Vault canvas adds them. If multiple items are dropped, all valid items are imported in one operation and progress is shown for larger files.

### 88.3 Item selection
Single click selects an item; double click opens it. Multi-select supports batch operations such as Move to Folder, Tag, Delete and Export.

### 88.4 Preview
Images and PDFs show an inline preview where practical. Video/audio shows a playable preview. Unsupported file types remain valid file cards with filename and file actions.

### 88.5 Text notes
Text notes open in a simple editor. Autosave is active. User can close and reopen without a separate Save step.

### 88.6 URL items
The URL itself is authoritative. Preview metadata, when retrieved, is secondary. If metadata retrieval fails, the URL remains usable.

### 88.7 Collections
Collections are user-created visual groupings and may contain references to items from different folders. Removing an item from a collection does not delete the item.

### 88.8 Pins
Pin changes ordering/display preference only. It does not move files.

### 88.9 Project copy
Copying a Global Vault item into a project creates a separate project item. The UI should use language such as “Copy to Project,” not “Sync to Project.”

### 88.10 Archive-like use
The user may keep old ideas indefinitely. There is no automatic cleanup based on age or “unused” status.


# 89. Story Board — Granular Functional Contract

### 89.1 Canvas creation zones
The Story Board should make it visually obvious where a new Act, Sequence, Beat or Scene can be dropped. Drop zones should be stable enough that users can predict the result before releasing the mouse.

### 89.2 Card expansion
Double-click opens the expanded card. Clicking outside saves changes and returns to board view unless the user explicitly cancels.

### 89.3 Reordering precision
When dragging, show a vertical insertion marker for sibling insertion and a container highlight for re-parenting.

### 89.4 Dragging a sequence
Moving a Sequence carries its Scene Cards and contained Beats. The objects keep their identities.

### 89.5 Dragging an Act
Moving an Act carries all descendants.

### 89.6 Multi-select move
If several cards are selected, dragging one selected card moves the group while preserving internal order.

### 89.7 Board zoom
Zoom affects presentation only. It must not change card order or size permanently unless the user chooses to save view settings.

### 89.8 Board filtering
Optional filters may hide cards by type or status but must not remove them from the story.

### 89.9 Parking Lot
Parking Lot is available from all board views. Moving an object there removes it from active story flow but retains content and parent history.


# 90. Scene Card — Exact Lifecycle

| Stage | User action | Result |
|---|---|---|
| New | Add Scene | New card inserted and focused |
| Edit | Type description | Card updates and auto-saves |
| Detail | Double click | Expanded editor opens |
| Move | Drag | Parent/order changes |
| Duplicate | Duplicate | New independent card |
| Park | Drag to Parking Lot | Removed from active flow |
| Restore | Drag back | Returns to selected position |
| Build | Include in Build Screenplay | Used to create screenplay scene |
| Delete | Delete | Soft-deleted/recoverable |
| Restore delete | Restore | Returns to former or unassigned location |

**Important:** Card deletion does not delete any screenplay scene that was previously created from it. The card and screenplay are separate objects after conversion.

### 90.1 Card description editing
The short description is plain text. It supports line breaks in expanded view if useful, but collapsed cards show the first meaningful lines/characters and truncate visually.

### 90.2 Heading editing
The scene heading is optional on the card. It is stored as a proposed heading, not a production scene heading, until the scene is created in a screenplay.

### 90.3 Attachments
Attachments can be added to the expanded card. They are reference material and do not become production assets automatically.


# 91. Build Screenplay — Detailed User Flow

1. User opens Story Board.
2. User selects Build Screenplay.
3. Application identifies active Scene Cards in board order.
4. Application shows a build preview.
5. Cards without a heading are marked “Heading needed.”
6. User can supply headings inline, exclude cards, or cancel.
7. User chooses a destination: New Screenplay or New Draft in an existing screenplay.
8. Application creates screenplay scenes in order.
9. Scene numbers are generated by screenplay order.
10. User lands in the new screenplay at the first created scene.

### 91.1 Rebuild warning
If the project already has a screenplay, “Build Screenplay” must explain that a new draft/document will be created and will not overwrite the current script.

### 91.2 Description treatment
A Scene Card description is planning material, not automatically screenplay prose. The build flow should offer a choice such as:

- Use description as scene planning note (default).
- Insert as temporary action text.

If temporary action text is selected, it is clearly marked as editable draft text rather than treated as final prose.

### 91.3 Included order
The source order is exactly the Story Board order at the moment of build. Parking Lot objects are excluded by default.


# 92. Screenplay Editor — Editing Contract

### 92.1 Cursor and selection
The editor supports standard text cursor, selection, copy, cut, paste and undo behavior.

### 92.2 Paragraph/element boundaries
Each screenplay element is a meaningful editable unit. The user can convert an element to another supported type using the element selector.

### 92.3 New scene
New scene is inserted at the current position or through scene navigator. The user supplies the heading; scene number is generated.

### 92.4 Scene navigation
Scene navigator updates after scenes are added/removed/reordered. Clicking an item moves focus without changing document state.

### 92.5 Section navigation
Where useful, the editor may display act/sequence outline context from Story Board, but this is reference-only unless the user explicitly edits through the Story Board.

### 92.6 Draft selector
Selecting another named draft opens that draft. If unsaved changes exist, the application completes save or requests a decision before switching.

### 92.7 Notes and comments
Toggling notes/comments changes visibility only. It does not insert them into screenplay content.

### 92.8 Focus mode
Focus Mode hides optional panels and toolbars but keeps typing, search, undo/redo, save and essential navigation available.


# 93. Script Review Package — Detailed Functional Flow

### 93.1 Create package
From Draft History or Review, select Share/Exchange → Review Package.

### 93.2 Selection
User chooses whole draft, selected scenes, or selected comments. The preview reports what will be included.

### 93.3 Export
Package is written to a user-chosen path. Export does not change the draft status.

### 93.4 Import
Receiver selects Import Exchange. OpenFrame identifies package type and source draft/version.

### 93.5 Mapping
The importer attempts to map comments to:

- same scene identity;
- same textual anchor;
- same scene heading and local position when identity is unavailable.

### 93.6 Ambiguous mapping
Ambiguous comment mappings are placed in a Review Queue. The user chooses the target scene/text manually.

### 93.7 Rejected package
Cancel leaves the local project unchanged and stores no partial review.


# 94. Comment Anchoring and Persistence

### 94.1 Text comment
Comment stores the selected text context, owning screenplay scene and surrounding context sufficient to relocate it after later edits.

### 94.2 Scene comment
Scene-level comment remains attached to scene identity.

### 94.3 Card comment
Comment remains attached to the Story Board object even after reordering or moving containers.

### 94.4 Deleted target
If the target object is deleted, the comment remains in history and points to the deleted object until permanently purged.

### 94.5 Resolved comment
Resolving does not delete. The comment remains accessible through review history.

### 94.6 Reply
Replies append to the same comment thread and do not create separate review items.


# 95. Script Lock — Production Safety Contract

Locking is an explicit workflow boundary.

### 95.1 Preconditions
No formal requirement exists that all comments be resolved. The lock dialog shows unresolved comments so the user can make an informed choice.

### 95.2 Lock result
The selected draft becomes the Shooting Draft reference. A lock timestamp and user identity are recorded.

### 95.3 Editing locked content
Typing into locked content triggers Start Revision. The locked source itself remains unchanged.

### 95.4 Revision creation
Revision is a new writable document derived from the locked source. The user can name it and optionally select revision color.

### 95.5 Production source selection
Breakdown can reference the locked draft or a later approved revision. The chosen source is always visible.


# 96. Breakdown — Detailed Screen and Interaction Contract

### 96.1 Scene list
Scenes display generated screenplay numbers, headings and concise status indicators.

### 96.2 Script reading pane
The user can read the screenplay scene while tagging elements without leaving the Breakdown page.

### 96.3 Tagging from text
Selecting relevant script text and choosing a category creates a suggested or manual element depending on entry mode.

### 96.4 Manual element
User can click Add Element and enter item name without selecting text.

### 96.5 Reuse
When typing an item name, existing catalog matches appear immediately. User may choose an existing item or create new.

### 96.6 Category visibility
Categories with confirmed content appear first. Empty categories can remain collapsed.

### 96.7 Scene completion
A scene can be marked Breakdown Complete manually. The status is a planning indicator, not a claim that every real-world requirement has been discovered.


# 97. Breakdown Suggestion Decision Rules

Automatic suggestions must follow this sequence:

```text
Analyze selected scene
      ↓
Produce candidate elements
      ↓
Show category + matched context
      ↓
User Accept / Edit / Reject
      ↓
Confirmed production data
```

### 97.1 Existing item match
If a suggestion resembles an existing catalog item, show the match as the default choice but do not silently attach it.

### 97.2 False positive
Rejecting a suggestion affects the current suggestion set; it does not delete a catalog item or prevent the same word from being used elsewhere.

### 97.3 Manual correction
User can change the category before acceptance.

### 97.4 Batch accept
Batch accept can be supported for obviously grouped suggestions, but the UI must show the exact count/categories before committing.

### 97.5 AI provenance
AI-assisted suggestions are labeled as suggestions. After confirmation they become ordinary production data and should not clutter the scene with AI branding.


# 98. Production Catalog — Cross-Scene Behavior

### 98.1 Single identity
A catalog item such as `Red Motorcycle` is one production identity even when required by ten scenes.

### 98.2 Scene association
Each scene records that it requires the catalog item.

### 98.3 Item edit
Changing catalog description/image/status updates the item wherever it is displayed. It does not alter historical screenplay text.

### 98.4 Item replacement
User may replace the catalog association for selected scenes without deleting the old item.

### 98.5 Archive
Archiving an item removes it from normal active selection while keeping historical scene associations readable.


# 99. Location Workflow — From Idea to Confirmed Location

1. User creates a Location or creates one while breaking down a scene.
2. Location starts at Idea or Shortlisted.
3. User adds address/photos/contact/notes.
4. User sees all scenes needing the location.
5. User changes status to Confirmed after real-world confirmation.
6. Schedule can use the confirmed location.
7. Call Sheet pulls name/address and other selected information.

Changing the location status does not alter the screenplay.


# 100. Cast/Crew Workflow — Minimal but Functional

### 100.1 Cast creation
Create actor/person, connect to character, add contact and availability notes.

### 100.2 Character linkage
Character identity remains the story-level object. Actor is the real-world person attached to the character.

### 100.3 Availability notes
Availability is freeform/lightweight. It can be referenced during scheduling.

### 100.4 Crew
Crew members are grouped by department and can appear on call sheets depending on the shooting day.

### 100.5 Removal
Removing an actor/crew member from directory does not remove historical references from exported documents. The system preserves historical document snapshots.


# 101. Moodboard — Exact Interaction Contract

### 101.1 Create board
User creates a board with a name. No board category is required.

### 101.2 Add items
Drop image/file, paste image, add text note or URL.

### 101.3 Move item
Drag changes position. Multi-select permits group movement.

### 101.4 Resize
Resize affects visual layout only.

### 101.5 Caption
Caption is optional and remains attached to the visual item.

### 101.6 Private notes
Private notes are excluded from standard moodboard exports.

### 101.7 Export
User selects layout/export. The result is a snapshot; editing the exported file does not alter the board.


# 102. Storyboard — Exact Interaction Contract

### 102.1 Scene entry
User selects a screenplay scene and chooses Create Storyboard, or starts from Production → Storyboards.

### 102.2 Panel creation
A new panel is created with an empty visual area and short description.

### 102.3 Visual source
User imports an image, uses a drawing/sketch tool if present, or leaves a blank placeholder.

### 102.4 Shot association
Panel may be attached to a Shot. If a shot already has a storyboard panel, creating another panel creates a second option rather than replacing the first.

### 102.5 Ordering
Panel order controls displayed shot/panel numbering within the storyboard.

### 102.6 Export
Export supports one scene, selected scenes, or full storyboard board where available.


# 103. Shot List — Exact Interaction Contract

### 103.1 Scene selection
Shot List opens to a selected scene or the full project shot list.

### 103.2 Add Shot
Add Shot creates a card/row at the end of the selected scene coverage.

### 103.3 Numbering
Shot number is generated from scene identity and shot order. Users do not manually renumber after dragging.

### 103.4 Optional technical details
Advanced camera fields remain optional. The user may create a useful shot with only description.

### 103.5 Coverage grouping
Shots can be grouped by scene. Optional labels such as “master”, “coverage” or “insert” can be entered as freeform notes rather than creating a large taxonomy.

### 103.6 Export
Export is optimized for camera/lighting/AD reference: readable shot number, description and selected technical fields.


# 104. Stripboard — Exact Interaction Contract

### 104.1 Create schedule
The schedule is tied to a selected screenplay source.

### 104.2 Unscheduled pool
All relevant screenplay scenes begin unscheduled.

### 104.3 Create shooting day
User enters date and optional note. A day strip/column is created.

### 104.4 Add scene to day
Drag strip into day. The day updates automatically with scenes, locations, cast and estimates.

### 104.5 Reorder scene in day
Drag within day. This changes shooting order only.

### 104.6 Move scene to another day
The scene leaves the original day and enters the destination. Any generated call sheet for either affected day is marked as potentially stale.

### 104.7 Remove from schedule
Drag back to Unscheduled or use Remove from Day.

### 104.8 Off day
An off day contains no screenplay scenes and can be used for rest/logistics. It can have notes.

### 104.9 Day duration
User can define a simple target duration. If estimates exceed it, show a warning.


# 105. Call Sheet — Exact Functional Contract

### 105.1 Generate
Create Call Sheet uses one shooting day as source.

### 105.2 Prefill
Populate title/date/day/scenes/locations/cast and selected notes automatically.

### 105.3 Missing data
Missing call times or contact information show as blank placeholders with a clear “needs input” marker rather than blocking document creation.

### 105.4 User edits
Document-level edits are local to that call sheet unless the user explicitly updates the underlying source record.

### 105.5 Refresh
If schedule or source data changes, call sheet shows “Source changed.” User clicks Refresh and previews differences before applying them.

### 105.6 Finalize
Finalized call sheet becomes a named snapshot. A later change creates a new call-sheet revision/snapshot rather than overwriting the issued PDF.


# 106. Project Notes, Tasks and Activity History

These are supporting features and must remain lightweight.

### 106.1 Project Notes
A project can contain general notes not tied to Idea Vault, Story or Production. Examples: meeting notes, “things to decide,” or production reminders.

Project Notes have title/body, optional tag and optional attachment.

### 106.2 Lightweight Tasks
Tasks are simple:

- title;
- optional due date;
- optional owner;
- status Open / Done;
- optional related object.

Tasks are not a full project-management system.

### 106.3 Related task
A task may be attached to Scene, Location, Character, Breakdown Item, Shooting Day or Call Sheet.

### 106.4 Completion
Marking a task Done does not change the related object automatically.

### 106.5 Activity History
Activity history records meaningful project actions such as:

- Draft created;
- Scene moved;
- Script locked;
- Breakdown source changed;
- Schedule day created;
- Call sheet issued;
- Exchange imported.

Normal character-by-character typing should not flood activity history.


# 107. Research / Reference Boards

Research boards are structured enough to keep research discoverable but intentionally less formal than a knowledge-management system.

### 107.1 Create research board
Name required. Optional description.

### 107.2 Add source
Support note, URL, PDF, image, document or quote.

### 107.3 Research note
User can write why a source matters.

### 107.4 Link to project element
Optional link to Scene Card, screenplay scene, Character, Location or production item.

### 107.5 Verification flag
Optional simple values: Unchecked / Checked / Needs Expert Check.

### 107.6 No automatic factual claims
The app does not label research “true” merely because it has been imported. Verification remains user-controlled.


# 108. Production Dashboard — Functional Rules

The Production Dashboard is a compact status surface.

It may show:

- current script source;
- breakdown completion count;
- schedule completion count;
- upcoming shooting day;
- unresolved location/cast needs;
- latest call sheet state.

Every dashboard item is clickable and leads to the source workspace.

Dashboard numbers must not become prerequisites. A user can continue working even if a metric is incomplete.


# 109. Lightweight Budget — Detailed Boundaries

### 109.1 Project budget
Budget is optional and may be ignored entirely.

### 109.2 Line item
Each line item contains description and amount, plus optional category.

### 109.3 Category total
Category totals calculate from line items.

### 109.4 Contingency
Contingency is a simple percentage or amount field. User controls value.

### 109.5 Editing
Budget values do not automatically update due to scene/production changes. The dashboard may show a manual “Review budget” reminder if a major planning change occurs, but the system cannot mutate costs silently.

### 109.6 Export
A simple budget summary can be exported as PDF/CSV-style data if implemented in the release.


# 110. Production Reports — Functional Set

Only practical reports belong here.

### 110.1 Scene report
List scenes with heading, page count and selected production indicators.

### 110.2 Location report
List locations with scenes and status.

### 110.3 Cast scene report
Actor → character → scenes.

### 110.4 Prop report
Catalog prop → scenes.

### 110.5 Schedule report
Shooting day → date → scenes → locations.

### 110.6 Breakdown completeness
Show confirmed/suggested/unreviewed elements by scene.

Reports are snapshots generated on demand. They do not become their own persistent database layer unless the user saves/export them.


# 111. Sides — Functional Contract

### 111.1 Selection
User selects shooting day.

### 111.2 Scene set
Sides contain the scenes scheduled for that day from the chosen production source.

### 111.3 Optional cover
Call-sheet information can be included as a cover page when user chooses it.

### 111.4 Revision state
The sides document displays the screenplay source/draft name so recipients know which script version they are reading.

### 111.5 Export
PDF snapshot.


# 112. File and External Storage Behavior

### 112.1 Project on removable drive
If project is stored on an external drive, the app continues normally while the drive is available.

### 112.2 Drive unavailable
When a project is open and its storage becomes unavailable, the application warns immediately, preserves unsaved state as safely as possible, and provides a reconnect/retry path.

### 112.3 External reference
Where the user chooses to reference a file outside the project, the application stores its location and displays a reference indicator.

### 112.4 Portable copy
User can choose to collect referenced files into a portable project copy where supported.

### 112.5 Duplicate external file name
The application uses project context rather than filename alone when resolving references.


# 113. Template Application Behavior

### 113.1 Create from template
User selects a starter template during New Project or from Templates.

### 113.2 Template content
A template may include:

- Story Board sample structure;
- call sheet layout;
- moodboard layout;
- basic budget categories;
- project notes.

### 113.3 No accidental template sharing
A project created from a template is independent. Editing the template later does not modify it.

### 113.4 Reset template
Resetting a template affects future uses only.


# 114. Beginner and Professional Workflow Switching

### 114.1 Beginner path
A new project can be used with a recommended visible progression:

Idea Vault → Story Board → Screenplay → Breakdown → Production → Schedule → Call Sheet.

### 114.2 Experienced path
Import Screenplay bypasses the Story Board. Breakdown can be opened immediately. Schedule can be built without Moodboard or Storyboard.

### 114.3 No mode lock
The user can move between workflows at any time. “Beginner” is not a permission level.

### 114.4 Guidance
Contextual helper text may explain a feature the first time it is opened. Help text can be dismissed and must not block work.


# 115. Project Status and Phase Indicators

Project phase is a user-controlled label.

### 115.1 Status selection
User may change status from Project Home.

### 115.2 Status transition
Changing phase does not create required tasks or disable features.

### 115.3 Display
Status appears in project header and recent project row.

### 115.4 Automatic suggestion
The application may suggest a status transition when obvious, but the user must accept it.


### Contextual Attention Indicators

OpenFrame uses lightweight contextual attention indicators, not notification spam.

Examples:

- “Call Sheet needs refresh.”
- “3 scenes need breakdown review.”
- “Production source changed.”
- “2 exchange comments waiting.”
- “Schedule conflict detected.”

Indicators should be attached to the relevant workspace and disappear once resolved or dismissed.

# 117. Import/Export Round-Trip Expectations

Functional round-trip rules:

### FDX
OpenFrame-exported FDX should be re-importable with screenplay structure substantially intact.

### Fountain
OpenFrame-exported Fountain should be parseable back into equivalent screenplay elements.

### DOCX
Export/re-import should preserve readable screenplay content even if some layout details vary.

### PDF
PDF is primarily a presentation/distribution format. Round-trip to editable screenplay is best-effort and must show parse warnings when structure is uncertain.

### Exchange packages
Exchange package round-trip must preserve the package's defined source identity and comments/objects without requiring cloud access.


# 118. Project Integrity and Recovery Functional Requirements

### 118.1 Crash recovery
If the application closes abnormally, reopening the project should detect recoverable edits.

### 118.2 Recovery choice
User sees a concise recovery dialog with:

- Recover latest state;
- use last confirmed save;
- dismiss recovery copy.

### 118.3 Backup before risky migration/import
Operations that can substantially change a project, such as importing a full exchange or project package into an existing project, should offer to create a backup first.

### 118.4 Integrity warning
If project content cannot be fully read, the application must state which area is affected and avoid presenting partial data as complete without warning.


# 119. Local Collaboration Session — Functional State Model

Session states:

`Not Running → Host Starting → Running → Participant Joined → Participant Active → Participant Disconnected → Session Ended`

### 119.1 Host start
Host chooses project and collaboration scope.

### 119.2 Participant join
Participant sees project/session name and granted role.

### 119.3 Shared editing
Changes in shared scope become visible to connected participants.

### 119.4 Private areas
Private notes remain private even during a shared project session unless explicitly converted to shared content.

### 119.5 Session end
The shared session stops. Each machine retains its local project state according to the agreed final state.


# 120. Local Collaboration Locking and Presence

### 120.1 Presence
When a user is actively editing an object, other users may see a simple presence indicator.

### 120.2 Soft lock
For high-conflict areas such as screenplay text or complex schedule rearrangement, a soft lock may indicate “X is editing this.” Other users can request access or continue if the workspace permits concurrent editing.

### 120.3 No permanent lockout
A user disconnecting must not leave a permanent lock. Locks expire when the session knows the participant is gone and can also be released by an authorized host.

### 120.4 Local-only
Lock state exists only in the collaboration session; it is not written as a permanent project restriction.


# 121. Portable Exchange — User-Visible Comparison Before Apply

Before applying an exchange package, show a comparison summary.

Example:

```text
14 comments
3 new scene cards
1 changed scene description
2 new shots
0 deletions
```

The user can expand each category.

Apply options should be granular where practical:

- import all;
- comments only;
- selected items;
- cancel.

This is preferable to an all-or-nothing merge whenever the package contains mixed content.


# 122. Export Selection Rules

Every export dialog should make the scope explicit.

Examples:

- Current scene
- Selected scenes
- Current sequence
- Entire screenplay
- Current shooting day
- Entire schedule

The dialog must display the source version and destination format before the user confirms.

If private content is excluded, the export summary should state “Private notes excluded.”


# 123. Printing and Document Layout Behavior

Printed documents must be generated from the same content model used by PDF export.

### 123.1 Page headers/footers
Where appropriate, include project title and page information.

### 123.2 Page breaks
Documents should avoid breaking a single logical heading/label from its immediate content where practical.

### 123.3 Printer-safe margins
Standard print margins are part of the document layout specification.

### 123.4 Print preview
Before printing, show a preview using the same output as the generated PDF.


# 124. Cross-Module Change Impact Rules

The application must distinguish **derived display updates** from **source-data mutation**.

| Change | Allowed automatic effect | Forbidden automatic effect |
|---|---|---|
| Story Board card reorder | Update board order | Rewrite prose silently |
| Story Board card edit | Update card | Rewrite screenplay |
| Screenplay scene heading edit | Update screenplay | Rename location catalog silently |
| Screenplay source update | Flag production review | Delete breakdown/shot/storyboard data |
| Catalog status edit | Update catalog displays | Rewrite script |
| Schedule move | Update day totals | Rewrite screenplay |
| Schedule date edit | Update day | Overwrite issued call-sheet snapshot |
| Call-sheet edit | Update call-sheet document | Rewrite master schedule |
| AI suggestion | Show preview | Mutate project silently |


# 125. Functional Rules for Stale Objects

An object becomes **stale** when its source has changed since the object was generated/prepared.

Examples:

- Call Sheet source schedule changed.
- Shot List scene changed.
- Breakdown source script changed.
- Review package based on an older draft.

A stale indicator must say:

1. what changed;
2. when it changed;
3. what action is available.

Example:

> “Scene 24 changed in Shooting Draft B. Review Breakdown.”

Stale does not mean invalid. The user can continue using the stale object until they decide whether to refresh it.


# 126. Functional Rules for Historical Snapshots

Exported/issued documents are historical snapshots.

Examples:

- Issued Call Sheet 2027-06-14 v1 remains unchanged when the schedule later changes.
- Exported Shooting Draft PDF remains the exact exported file even if the project moves to Revision A.
- Exchange package remains a snapshot of its source state.

The application can display newer related versions but must not rewrite the historical artifact.


# 127. Cross-Cutting Functional Constraints

> These requirements apply to all features but do not constitute independent product modules.

## 127.1 Security and Privacy — Functional Behavior

This section defines user-visible privacy behavior; implementation details belong elsewhere.

### 127.1 Local project ownership
A project stored locally remains readable only through the user's machine/file access unless the user chooses to share/export it.

### 127.2 AI disclosure
When using an external AI provider, the UI must clearly state that selected/project information is being sent outside OpenFrame.

### 127.3 Private note exclusion
Private notes must not appear in standard exports, exchange packages or call sheets.

### 127.4 Permission visibility
When collaborating, the user can see their own current role and the scope of access granted.


## 127.2 Accessibility and Input Flexibility — Functional Requirements

The desktop app must not assume mouse-only use.

Functional expectations:

- keyboard focus can move between major controls;
- important commands have keyboard shortcuts where practical;
- drag/drop has a non-drag alternative where necessary;
- forms expose labels to assistive technologies;
- color is not the only indicator of state;
- selected/focused elements are visibly distinguishable.

This is a behavior requirement; exact implementation resides in the UX/accessibility specification.


## 127.3 Performance-Oriented Functional Expectations

These are user-perceived expectations rather than implementation targets.

### 129.1 Normal editing
Typing in a screenplay, note or card should remain responsive.

### 129.2 Large boards
A board with hundreds of scene cards remains navigable and does not require the user to split the project into multiple apps.

### 129.3 Large screenplay
A long feature screenplay remains searchable and editable as one document.

### 129.4 Large Vault
A large Idea Vault remains browsable through list/grid/folder views and search.

### 129.5 Large schedule
A schedule with many shooting days remains manageable in board/list views.


# 130. Complete Module Dependency Matrix

|Module|Requires|Produces/Feeds|Must not mutate|
|---|---|---|---|
|Global Idea Vault|Local project/user|Project Idea Vault copies|Project/story/script|
|Project Idea Vault|Project|Story copies|Screenplay automatically|
|Story Board|Project|Optional screenplay build|Existing screenplay silently|
|Characters|Project|Screenplay/Production references|Screenplay prose|
|Screenplay|Project|Breakdown, shots, storyboard, schedule source|Idea Vault automatically|
|Breakdown|Screenplay source|Catalog|Script text|
|Catalog|Project/breakdown|Schedule/Call Sheet data|Screenplay|
|Locations|Project/catalog|Schedule/Call Sheet|Screenplay|
|Cast & Crew|Project/characters|Schedule/Call Sheet|Screenplay|
|Moodboards|Project|Exports/references|Production data|
|Storyboard|Scene/optional shot|Shot List/reference|Screenplay prose|
|Shot List|Scene|Call Sheet/reference|Screenplay prose|
|Schedule|Screenplay/catalog|Call Sheets|Issued call sheets|
|Call Sheet|Shooting Day|PDF/export|Master schedule|
|AI|Selected context|Suggestions/answers|Anything automatically|
|Exchange|Selected workspace|Portable review/update package|Host project automatically|


# 131. Full Acceptance Scenario Set

The following scenarios form a minimum end-to-end functional test suite. Each scenario should eventually become one or more QA test cases.

### A. Idea and development
1. Create global idea → close → reopen → content remains.
2. Add image + note → switch views → same item remains.
3. Copy global idea into project → edit copy → global original unchanged.
4. Move project idea to Story → original remains in Vault.
5. Create Act → Sequence → Beat → Scene.
6. Move Scene between sequences.
7. Move Sequence between acts.
8. Send Scene to Parking Lot and restore.
9. Duplicate Scene and edit duplicate.

### B. Screenplay
10. Build script from Story Board.
11. Build script with missing headings and resolve them.
12. Import FDX.
13. Import PDF with warning.
14. Import DOCX.
15. Paste screenplay text.
16. Export PDF/FDX/Fountain/DOCX.
17. Create Draft 2.
18. Compare Draft 1 and Draft 2.
19. Add comment to text.
20. Resolve comment.
21. Lock shooting draft.
22. Attempt edit and start revision.
23. Create revision color.

### C. Production
24. Create Breakdown.
25. Manually tag prop.
26. Suggest breakdown elements.
27. Reject a suggestion.
28. Accept a suggestion using existing catalog item.
29. Create new location.
30. Assign cast.
31. Create moodboard.
32. Create storyboard.
33. Create shots.
34. Link storyboard panel to shot.
35. Build schedule.
36. Drag scenes to days.
37. Trigger cast/location conflict warning.
38. Create call sheet.
39. Change schedule and mark call sheet stale.
40. Refresh call sheet.
41. Export sides.
42. Export schedule.

### D. Offline/ownership
43. Disconnect network and edit project.
44. Save/reopen offline.
45. Export project package.
46. Restore backup.

### E. Collaboration
47. Export screenplay review package.
48. Import response package.
49. Import stale response.
50. Resolve ambiguous comment mapping.
51. Start local-network session.
52. Join second desktop.
53. Edit different scenes concurrently.
54. Trigger same-text conflict.
55. Disconnect participant.
56. End session.

### F. Safety
57. Delete scene card with linked screenplay scene.
58. Delete catalog item referenced by scenes.
59. Cancel destructive operation.
60. Crash/reopen and recover autosave.


# 132. Change Management Rules for the Product Specification

Once implementation starts, any change to the following requires product review before engineering implementation:

- source-of-truth rules;
- object identity rules;
- screenplay version behavior;
- script-to-breakdown mapping;
- schedule-to-call-sheet behavior;
- offline ownership;
- collaboration package behavior;
- deletion/restore behavior;
- supported import/export formats;
- AI permission boundaries.

UI wording and layout can evolve without changing functional behavior, but changes that alter what data is authoritative or how a user action changes another module are product changes and must be versioned.


# 133. Definition of Functional Completeness by Module

A module is functionally complete only when all of these are specified and tested:

1. Entry point.
2. Empty state.
3. Creation.
4. Editing.
5. Selection.
6. Navigation.
7. Reordering/movement where applicable.
8. Duplication where applicable.
9. Delete/archive.
10. Restore.
11. Undo/redo.
12. Save/autosave.
13. Offline behavior.
14. Permission behavior.
15. Export.
16. Import where applicable.
17. Cross-module connections.
18. Source-of-truth rule.
19. Stale/conflict behavior.
20. Error behavior.
21. Acceptance criteria.

A team should not mark a module “done” if it only demonstrates the happy-path screen.


# Appendix E — Requirement Traceability Ledger

|PRD ID|Source requirement area|FSD handling|QA expectation|
|---|---|---|---|
|0|Document Purpose|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|1|Product Thesis|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|2|Product Philosophy|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|3|Target User|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|4|Users and Project Roles|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|5|Platform Definition|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|6|Product Information Model|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|7|Application Shell|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|8|Project Lifecycle|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|9|Project Creation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|10|Global Idea Vault|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|11|Idea Vault Item Behavior|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|12|Idea Vault Organization|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|13|Moving From Idea Vault Into Story|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|14|Project Idea Vault|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|15|Project Home|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|16|Story Workspace|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|17|Story Board Views|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|18|Acts|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|19|Sequences|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|20|Beat Cards|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|21|Scene Cards|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|22|Scene Card Numbering|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|23|Scene Card Expansion|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|24|Drag-and-Drop Story Editing|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|25|Parking Lot|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|26|Duplicate and Experiment|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|27|Story Board to Screenplay|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|28|“Build Screenplay From Story Board”|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|29|Story Board and Screenplay Relationship|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|30|Characters|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|31|Character Relationship View|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|32|Story Timeline|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|33|Screenplay Workspace|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|34|Screenplay Element Types|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|35|Intelligent Element Navigation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|36|Screenplay Layout|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|37|Scene Navigation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|38|Normal Find/Search|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|39|Screenplay Notes|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|40|Writing Room Side Panels|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|41|Idea Vault Is Not a Live Writing Sidebar|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|42|Screenplay Import|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|43|Screenplay Import Flow|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|44|Import Safety|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|45|Screenplay Export|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|46|Script Drafts|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|47|Draft Creation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|48|Draft Comparison|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|49|Review Rounds|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|50|Comments|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|51|Private Notes|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|52|Script Lock|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|53|Locked Script Behavior|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|54|Production Revision Colors|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|55|Episodic Projects|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|56|Series-Level Story Board|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|57|Breakdown|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|58|Breakdown Principle|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|59|Automatic Breakdown Suggestions|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|60|Breakdown Categories|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|61|Production Catalog|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|62|Catalog Item Details|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|63|Locations|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|64|Location Notes|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|65|Cast|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|66|Crew|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|67|Moodboards|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|68|Moodboard Export|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|69|Storyboards|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|70|Script-to-Storyboard|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|71|Shot Lists|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|72|Shot Card|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|73|Shot List / Storyboard Relationship|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|74|Scheduling — Stripboard|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|75|Stripboard Views|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|76|Schedule Creation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|77|Shooting Days|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|78|Schedule Assistance|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|79|Basic Schedule Conflict Detection|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|80|Schedule Estimation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|81|Schedule Calendar|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|82|Call Sheets|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|83|Call Sheet Required Information|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|84|Call Sheet Editing|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|85|Call Sheet Export|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|86|The User's Collaboration Problem|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|87|Collaboration Model — Three Modes|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|88|Exchange Package Principle|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|89|Review Package|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|90|Merge Safety|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|91|Page-Specific Exchange Packages|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|92|Why Exchange Packages Are Important|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|93|AI Assistant|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|94|AI Assistant — Allowed Capabilities|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|95|AI Assistant — Not Allowed by Default|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|96|AI Scope Selector|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|97|AI Action Approval|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|98|Global Project Search|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|99|Files|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|100|Undo / Redo|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|101|Delete Behavior|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|102|Multi-Select|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|103|Keyboard Support|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|104|Drag-and-Drop Support|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|105|Project Saving|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|106|Project Portability|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|107|External Storage|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|108|Project Backup|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|109|Beginner Experience|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|110|Professional Experience|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|111|Progressive Disclosure|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|112|Exact Connection Map|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|113|Connections That Are NOT Required|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|114|Source of Truth Rules|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|115|Scene Identity|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|116|Scene Reordering Safety|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|117|Breakdown After Script Change|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|118|Schedule After Script Change|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|119|Call Sheet After Schedule Change|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|120|Production Dashboard|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|121|Project Status|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|122|Templates|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|123|Export Philosophy|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|124|Export Selection|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|125|Printing|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|126|Empty States|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|127|Error Philosophy|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|128|Safety Around Destructive Actions|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|129|Collaboration Permissions|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|130|Offline Collaboration State|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|131|Local Network Collaboration|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|132|Local Collaboration Locks|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|133|Remote Collaboration Without Cloud|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|134|Review Package UX|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|135|Comment Import Mapping|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|136|Story Board Exchange|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|137|Shot List Exchange|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|138|Breakdown Exchange|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|139|Why This Matters|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|140|Research / Reference Boards|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|141|Project Notes|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|142|Lightweight Tasks|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|143|Task Behavior|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|144|Activity History|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|145|No Mandatory Workflow|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|146|Feature Access Matrix|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|147|Screenplay-to-Production Handoff|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|148|Production Source Version|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|149|Production Source Update|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|150|Breakdown Reconciliation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|151|Shot List Reconciliation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|152|Storyboard Reconciliation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|153|Schedule Reconciliation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|154|Basic Production Reports|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|155|Sides|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|156|Daily Production View|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|157|What Happens After Shooting?|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|158|No Post-Production Expansion in Core|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|159|What We Are Not Building|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|160|Feature Bloat Guardrail|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|161|The Four Core Jobs|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|162|Primary User Journey — First Film|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|163|Primary User Journey — Existing Screenplay|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|164|Primary User Journey — Short Film|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|165|Primary User Journey — Writer Only|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|166|Primary User Journey — Producer/AD|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|167|Product Behavior Around Unfinished Work|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|168|Product Behavior Around Messy Creativity|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|169|Project Search vs Script Search|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|170|Contextual Navigation|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|171|Scene Hub|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|172|Sequence Hub|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|173|Project-level Quick Actions|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|174|Context Menus|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|175|Quick Capture|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|176|Idea Vault Favorites|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|177|Idea Vault Collections|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|178|Idea Vault Reuse|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|179|Project Archive|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|180|Recent Projects|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|181|Performance Expectations as Product Behavior|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|182|Media Handling Scope|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|183|Visual Consistency|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|184|Desktop Windowing|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|185|External Monitor Use|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|186|Fullscreen Writing Mode|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|187|Professional Document Identity|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|188|Page-Specific UX Philosophy|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|189|UX Rule — Don't Show All Data Everywhere|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|190|UX Rule — One Page, One Job|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|191|UX Rule — Never Force Studio Terminology|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|192|UX Rule — Let Short Films Stay Short|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|193|UX Rule — Don't Make a User Configure the Product Before Using It|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|194|UX Rule — The Product Should Teach Through Context|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|195|Feature Priority — P0|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|196|Feature Priority — P1|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|197|Feature Priority — P2|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|198|Explicitly Deferred|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|199|MVP Product Test|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|200|Acceptance Criteria — Idea Vault|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|201|Acceptance Criteria — Story Board|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|202|Acceptance Criteria — Screenplay|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|203|Acceptance Criteria — Breakdown|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|204|Acceptance Criteria — Shot List|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|205|Acceptance Criteria — Storyboard|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|206|Acceptance Criteria — Scheduling|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|207|Acceptance Criteria — Call Sheets|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|208|Acceptance Criteria — Collaboration|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|209|Acceptance Criteria — Offline|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|210|Acceptance Criteria — AI|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|211|Detailed Example — Idea to Scene|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|212|Detailed Example — Script to Breakdown|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|213|Detailed Example — Script to Shooting Plan|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|214|Detailed Example — Call Sheet|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|215|End-to-End Data Flow|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|216|Deliberate Separation of Creative and Production Data|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|217|Research Basis|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|218|Competitive Product Boundary|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|219|Product Success Definition|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|220|Final Product Definition|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|221|Non-Negotiable Product Rules|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|222|Product Design Decision: The Application Must Feel Smaller Than It Is|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|223|Product Design Decision: Avoid Feature Discoverability Overload|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|224|Product Design Decision: Filmmaker Ownership|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|225|Product Design Decision: A User Can Ignore Half the App|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|226|Product Design Decision: Every Feature Must Have a Clear Entry Point|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|227|Product Design Decision: Do Not Make the User Understand Data Models|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|228|Product Design Decision: Preserve Creative History|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|229|Final Scope Summary|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|230|Engineering Handoff Rule|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|
|231|Final Product Mantra|Expanded into one or more functional contracts above; source-to-behavior mapping retained.|At least one functional scenario or acceptance test exists for each applicable area.|


# Appendix F — Suggested Engineering Work Breakdown

This appendix does not prescribe code architecture. It defines a sensible implementation order so teams can build from stable foundations without prematurely coupling every module.

### Wave 1 — Local creative foundation

1. Application shell.
2. Project create/open/close.
3. Global/Project Idea Vault.
4. Story Board.
5. Acts/Sequences/Beats/Scene Cards.
6. Undo/redo.
7. Autosave/recovery.

### Wave 2 — Screenwriting

8. Screenplay data/editing.
9. Build from Story Board.
10. Drafts/history.
11. Comparison.
12. Comments/reviews.
13. Lock/revisions.
14. Import/export.

### Wave 3 — Production planning

15. Breakdown.
16. Catalog.
17. Locations.
18. Cast/Crew.
19. Moodboards.
20. Storyboard.
21. Shot List.

### Wave 4 — Scheduling and documents

22. Stripboard.
23. Conflict warnings.
24. Daily Production View.
25. Call Sheets.
26. Sides/reports.
27. Lightweight Budget.

### Wave 5 — Collaboration and intelligence

28. Exchange packages.
29. Local network session.
30. Conflict/reconciliation.
31. AI assistant.
32. Advanced import/export hardening.

This ordering is functional: it builds the core source objects before the downstream features that depend on them.


# Appendix G — Product Owner Review Checklist Before Release

Before each release, the product owner should verify:

### Creative
- Can a new filmmaker dump an idea without setup friction?
- Can they create acts/sequences/scenes quickly?
- Can they rearrange the outline without numbering work?
- Can they write or import a screenplay?
- Can they preserve/review drafts?

### Production
- Can the locked/current screenplay become a breakdown?
- Can confirmed elements become reusable catalog items?
- Can a small team plan visuals and shots?
- Can scenes become a practical stripboard?
- Can a call sheet be produced without retyping the day?

### Ownership
- Does the project still work with no internet?
- Can the project be backed up and moved?
- Can remote reviewers collaborate using files?
- Can local collaborators work together without OpenFrame cloud?

### Simplicity
- Are users being asked for information that has no current purpose?
- Has a new feature accidentally added another permanent sidebar item?
- Are cards still small and scannable?
- Can a short film ignore most of the app and remain comfortable?

### Safety
- Can creative experiments be undone?
- Can deleted work be recovered?
- Can stale production documents be recognized?
- Can a collaborator's work be lost silently?


**Expanded FSD size:** approximately 30,765 words / 4,628 lines.

# 134. Field-Level Functional Contract

This section defines the minimum user-visible fields for the application's major objects. The purpose is to prevent implementation teams from introducing hidden required fields or omitting a field needed by another workflow.

### 134.1 Project
| Field | Required | Editable | Used by |
|---|---|---|---|
| Title | Yes | Yes | All project views |
| Type | Yes | Yes | Workflow/context |
| Language | No | Yes | Project documents/metadata |
| Genre | No | Yes | Project reference |
| Creator | No | Yes | Project documents |
| Status | No | Yes | Home/header |
| Project notes | No | Yes | Home/Files |

### 134.2 Idea Vault Item
| Field | Required | Editable | Notes |
|---|---|---|---|
| Item type | Yes | Limited | Determines presentation |
| Title | No | Yes | May remain blank |
| Body/caption | No | Yes | Depends on type |
| File/URL | Depends on type | Limited | Primary source |
| Folder | No | Yes | Organization only |
| Collection(s) | No | Yes | Organization only |
| Tags | No | Yes | Search aid |
| Pin | No | Yes | Display aid |

### 134.3 Act
Required title; optional note.

### 134.4 Sequence
Required name; optional note. The name is intentionally the main field.

### 134.5 Beat
Required beat text; optional note, color and attachment.

### 134.6 Scene Card
Required short description; optional heading, notes and attachments. No manual scene number or production fields.

### 134.7 Character
Required name; optional role, description, image, notes and relationship notes.

### 134.8 Screenplay Scene
Required valid scene heading for normal screenplay output; action/dialogue/etc. are content, not metadata fields.

### 134.9 Draft
Required name; parent/source; optional note; state generated by workflow.

### 134.10 Review Round
Required source draft and name; optional reviewers/deadline; status generated/controlled.

### 134.11 Breakdown Element
Required category and name/reference; optional matched text and notes.

### 134.12 Catalog Item
Required name/category/status; optional image, description, notes and scene usage.

### 134.13 Location
Required name; optional address, contact, status, photos and practical notes.

### 134.14 Person
Required name; optional role, department, contact, image, availability and notes.

### 134.15 Shot
Required description; generated order/shot identifier; optional framing, movement, angle, lens, notes, storyboard reference.

### 134.16 Storyboard Panel
Required panel identity/order and a visual area; description optional but recommended; optional shot link and technical notes.

### 134.17 Shooting Day
Required date and day number; optional notes; scene membership derived from schedule.

### 134.18 Call Sheet
Required source shooting day; generated core production data; editable call times/notes/meeting point/attachments; revision state generated by workflow.


# 135. Page Action Matrix

|Page|Primary action|Secondary actions|Forbidden/hidden by default|
|---|---|---|---|
|Home|Open/Create Project|Archive, duplicate, rename, search|Production metadata|
|Idea Vault|Add|Search, filter, collection, copy to project, export|Required classification|
|Story Board|Add Scene/Add Sequence|Add Act/Beat, move, duplicate, park, build screenplay|Production fields|
|Characters|Add Character|Edit, archive, open scenes|Payroll/HR|
|Screenplay|Write/Edit|Drafts, compare, comment, export, lock|Idea Vault live sync|
|Breakdown|Tag/Suggest|Catalog, filter scenes, reports|Silent auto-confirm|
|Production|Open workspace|Catalog, locations, cast, visuals, schedule, budget|Enterprise modules|
|Shot List|Add Shot|Reorder, storyboard, export|Required camera metadata|
|Storyboard|Add Panel|Import/draw, reorder, export|Full illustration suite|
|Schedule|Create Shooting Day|Drag scenes, date, warning review, export|Forced auto-optimization|
|Call Sheets|Create Call Sheet|Edit, refresh, issue, export|Silent schedule mutation|
|Files|Add File|Move, rename, open, export, delete|Duplicate file bureaucracy|
|AI|Ask|Select context, accept/reject actions|Silent project changes|


# 136. Screen-Level State and Action Rules

### 136.1 Story Board state
Possible visible states: Empty, Populated, Filtering, Card Expanded, Dragging, Building Screenplay, Saving, Save Error.

When Dragging ends in a valid target, commit the move immediately. When Dragging ends outside valid targets, return the card to its original position with no data mutation.

### 136.2 Screenplay state
Possible visible states: Loading Draft, Editing, Searching, Comparing, Review Overlay, Locked, Revision Mode, Saving, Save Error.

A locked screenplay remains readable. Editing automatically transitions the user toward creating a revision rather than allowing silent changes to the locked baseline.

### 136.3 Breakdown state
Possible visible states: No Source, Loading Source, Reviewing Scene, Suggesting, Suggestions Ready, Scene Breakdown Complete, Needs Review.

### 136.4 Schedule state
Possible visible states: No Schedule, Unscheduled Pool, Planning, Conflict Warning, Day Selected, Export Preview.

### 136.5 Call Sheet state
Possible visible states: None, Draft, Source Changed, Ready, Finalized, Superseded.


# 137. Screenplay Import Mapping Rules

The following mapping rules describe expected functional interpretation; parser implementation is outside the FSD.

### 137.1 Scene heading
Text recognized as a screenplay scene heading becomes a Screenplay Scene heading.

### 137.2 Action
Ordinary action becomes Action elements.

### 137.3 Character
Character labels become Character elements.

### 137.4 Dialogue
Dialogue remains associated with the preceding character element.

### 137.5 Parenthetical
Parentheticals remain distinct where recognizable.

### 137.6 Transitions
Recognized transition text remains a Transition element.

### 137.7 Unknown formatting
Unknown structures remain readable. The system must not discard textual content merely because its formatting is unknown.

### 137.8 Scene count discrepancies
The import summary reports the number of detected scenes. If the user expected a different count, they can inspect the preview before committing.

### 137.9 Import into existing project
Always creates a new screenplay/draft unless the user explicitly selects a replacement workflow that the application presents as a destructive action.


# 138. Import Validation Checklist

Before completing screenplay import, display the following checks when applicable:

- File opened successfully.
- Text extracted.
- Scene headings detected.
- Characters detected.
- Page count estimated.
- Unsupported or ambiguous formatting identified.
- Potentially empty scenes identified.
- Original file remains untouched.

The user can continue with warnings. Warnings do not imply that the screenplay is unusable; they identify items worth checking.


# 139. Export Configuration Rules

Every export dialog must answer four questions before confirmation:

1. **What am I exporting?**
2. **Which version/source am I exporting?**
3. **What format am I exporting?**
4. **Where will the file be saved?**

Optional document settings are shown after these core choices. The default configuration should produce a professional usable result.

Private notes, comments and internal identifiers are excluded from normal deliverables unless the export is specifically a review package or internal report.


# 140. Revision Color Functional Rules

Revision colors are used only after a script has a production-oriented locked baseline.

### 140.1 Create revision
User chooses Start Revision from locked source.

### 140.2 Changed text
The system identifies changed content for the revision.

### 140.3 Revision color display
The revision color is applied to supported revision markers/pages according to the export mode.

### 140.4 No color in standard draft writing
Ordinary Draft 1/Draft 2 writing does not need production revision coloring.

### 140.5 Revision history
Every revision records its source, label, date and reason if provided.


# 141. Review Round Functional Matrix

|Action|Actor|Effect|Does not do|
|---|---|---|---|
|Start review|Owner/Editor|Creates review linked to selected draft|Does not lock script|
|Add comment|Reviewer/Commenter/Editor|Adds note to target|Does not mutate script|
|Reply|Any permitted reviewer|Appends reply|Does not resolve automatically|
|Resolve|Permitted reviewer/editor|Marks thread resolved|Does not delete|
|Complete review|Owner/Editor|Closes review round|Does not approve script unless explicit|
|Export review|Authorized user|Creates review package|Does not change source draft|
|Import review|Owner/Editor|Adds response/comment data after preview|Does not overwrite current draft|


# 142. Script Lock Functional Matrix

|Action|Result|Safety rule|
|---|---|---|
|Lock Draft|Selected draft becomes Shooting Draft|Explicit confirmation|
|Edit locked draft|Prompt to start revision|Locked source remains intact|
|Start Revision|New writable revision created|Source retained|
|Export locked draft|Snapshot exported|Export cannot mutate source|
|Delete locked draft|Blocked until another valid source exists or downgraded by authorized workflow|Prevent loss of sole production baseline|


# 143. Breakdown-to-Catalog Matching Rules

When a user adds a breakdown item, the application searches existing catalog items by relevant visible identity fields.

### Exact/obvious match
Show existing item as the first choice.

### Multiple plausible matches
Show matches with distinguishing context and allow user to choose or create new.

### No match
Offer Create New Catalog Item.

### Rename while attaching
If user changes a breakdown name before confirmation, the system re-runs matching rather than preserving an obsolete match.

### Existing item not appropriate
User can choose Create New even if a similar item exists.


# 144. Production Catalog Relationship Rules

The catalog is a directory of practical production identities.

A catalog item may be connected to many scenes.

Removing one scene association does not delete the catalog item.

Changing a catalog item name updates the production directory and its displays. Historical exported documents remain snapshots and are never rewritten.

Archiving a catalog item does not remove historical references.


# 145. Scheduling Field and Calculation Rules

|Value|Source|Editable in Schedule?|Behavior|
|---|---|---|---|
|Scene number|Screenplay order|No|Generated display value|
|Scene heading|Screenplay source|No in strip default; editable via source|Displays latest selected source|
|Page count|Screenplay source|Optional manual override if required|Override visibly marked|
|Location|Breakdown/catalog or screenplay heading|Yes for production assignment|Does not rewrite script|
|Cast requirement|Breakdown/character references|Derived|Used for conflict warnings|
|Estimated duration|User|Yes|Affects day total|
|Shoot date|Shooting Day|Yes|Used by call sheet|
|Day order|Schedule|Yes by drag|Changes production order only|


# 146. Schedule Conflict Behavior

### 146.1 Actor conflict
Show both affected scenes/days and the conflicting actor.

### 146.2 Location conflict
Show scenes requiring the same location in incompatible simultaneous placements where timing data exists.

### 146.3 Duration overflow
Show that the day's entered estimates exceed its target duration.

### 146.4 Missing information
Do not invent a duration or call time. Show Missing instead.

### 146.5 Resolution
Every warning provides:

- Open affected item;
- Keep schedule anyway;
- Move scene (user performs move);
- Dismiss warning where appropriate.

The system never makes the final scheduling choice for the user.


# 147. Call Sheet Source/Refresh Matrix

|Source change|Call sheet status|Default user action|
|---|---|---|
|Scene moved to another day|Needs Refresh|Preview changes|
|Scene removed|Needs Refresh|Review removal|
|New scene added to day|Needs Refresh|Preview addition|
|Cast assignment changed|Needs Refresh|Preview cast changes|
|Location changed|Needs Refresh|Preview location changes|
|Call-sheet-only note changed|No schedule effect|Continue|
|Schedule date changed|Needs Refresh|Preview new date|


# 148. Daily Call Sheet Finalization Workflow

1. Open shooting day.
2. Click Create Call Sheet.
3. Verify prefilled scenes, cast and locations.
4. Add call times and day-specific notes.
5. Save call sheet.
6. Optionally mark Final/Issued.
7. Export PDF.

If the schedule subsequently changes, the issued document remains historical. A new revision is produced when the user refreshes and reissues it.


# 149. File Package Taxonomy

|Package|Use|Contains|Import result|
|---|---|---|---|
|Full Project Package|Move/backup entire project|Project workspaces + project data + selected files|New local project or copy|
|Story Exchange|Story review|Acts/sequences/beats/scenes/selected notes|Review/copy/selected apply|
|Script Review|Screenplay feedback|Draft snapshot + comments/annotations|Review record/comments|
|Breakdown Exchange|Production breakdown review|Selected scene breakdown/catalog references|Proposed production updates|
|Shot Exchange|Shot planning|Shots + optional storyboard refs|Shot updates/copies|
|Schedule Exchange|Scheduling collaboration|Shooting days + scene assignments|Schedule review/selected apply|
|Call Sheet Review|Daily document feedback|Call sheet snapshot + comments|Call sheet review/comments|


# 150. Exchange Package Validation

On import, the application validates:

- package type;
- source project reference;
- source draft/version where applicable;
- package completeness;
- compatibility with current project;
- conflicts/staleness.

Then it shows a preview before applying anything.

If validation fails, no project mutation occurs.


# 151. Comment Mapping Decision Tree

When importing a review comment:

1. Does source object identity still exist? → map to it.
2. Does the exact text anchor still exist? → map to text.
3. Does the scene heading/context provide one clear match? → propose that match.
4. Are multiple matches possible? → put in Review Queue.
5. Is no safe match available? → retain comment as Unmapped Review Note.

Never silently discard a comment because mapping failed.


# 152. Local Network Collaboration Functional Session

### Host
Select project → Start Session → choose shared scope → assign participant roles → display join information.

### Participant
Join session → select identity/name → receive assigned role → enter shared workspace.

### During session
Users edit allowed objects. Presence indicators show who is active. Comments and project data follow normal object rules.

### Disconnect
Local changes remain on each machine. The session reports participant disconnected. The host continues working.

### Rejoin
Participant reconnects and receives current shared state, with unresolved conflicts presented rather than silently discarded.

### End
Host ends session. Each project remains independently usable offline.


# 153. Collaboration Scope Rules

Host can share:

- Entire project
- Story only
- Screenplay only
- Production only

If a user joins with Story-only access, they must not gain access to private screenplay/production content merely because the project is technically hosted.

Export-only participants cannot enter a live editing session.


# 154. Offline/Online UI Rules

The application should distinguish:

**Offline:** all core local functions available.

**Online:** optional external services available.

**Local Session:** connected to another OpenFrame desktop on the same private network.

**Cloud AI:** selected AI provider connection active.

These states should never be conflated.

Example status copy:

> `Offline · Local Project Saved`

or

> `Local Session · 2 collaborators connected`

or

> `AI Provider Connected`

No “Syncing to OpenFrame Cloud” state exists in the core product.


# 155. External AI Data Handling — User Flow

When a user invokes an external AI capability:

1. Determine selected context.
2. Show provider/context disclosure when required.
3. User submits request.
4. AI returns answer/suggestion.
5. User may accept a supported project mutation.
6. Mutation becomes a normal application action and is undoable.

If network access fails, the application reports that the AI action could not be completed and leaves all local project content unchanged.


# 156. AI Mutation Preview Contract

|AI request|Preview|Apply action|
|---|---|---|
|Create Scene Card|Proposed card text + parent sequence|Add to Story Board|
|Create Breakdown Elements|Suggested list grouped by category|Accept selected/all|
|Generate Synopsis|Proposed synopsis text|Copy into selected field|
|Suggest Schedule Grouping|Proposed grouping explanation|User manually applies or confirms|
|Draft Summary|Generated summary|Save as note/copy only|
|Rename/modify object|Exact proposed change|Confirm before mutation|


# 157. Activity History Rules

Activity History records meaningful events but not every keystroke.

Record examples:

- Project created.
- Draft created.
- Draft locked.
- Revision started.
- Scene card moved between sequences.
- Breakdown source changed.
- Catalog item created.
- Shooting day created.
- Call sheet finalized.
- Exchange imported/exported.
- Collaboration session started/ended.

Do not record ordinary typing as separate visible activity entries.

History may show actor, action, date/time and affected object. It is read-only from the UI.


# 158. Task System Functional Rules

Tasks are optional, small and contextual.

### Create task
Required title; optional due date, owner and related object.

### Complete task
Changes status to Done. Does not mutate the related object.

### Reopen
Returns to Open.

### Delete task
Soft delete/recoverable.

### Related task navigation
Click task relation → opens associated object.

### Task lists
User can see project tasks and optionally filter by owner/status.

There is no dependency graph, Gantt system, time tracking, sprint board or enterprise project management layer.


# 159. Project Notes Functional Rules

Project Notes are general freeform notes that do not belong naturally in Idea Vault or another module.

They support:

- title;
- body;
- attachment;
- optional related object.

Project Notes can be exported/printed if desired. They are excluded from formal documents by default.


# 160. Source Version Banner Rules

Any production workspace derived from a screenplay should display the source in a compact banner.

Example:

> `Production Source: Shooting Draft 6`

If newer approved script material exists, the banner may show:

> `Newer revision available — Review Production Update`

The banner is informational until the user chooses an update.


# 161. Production Update Review Flow

1. User opens Production.
2. Application sees newer script revision.
3. User chooses Review Update.
4. System compares source versions.
5. Impact list appears by scene.
6. User chooses Review each / Accept all safe mappings / Dismiss.
7. New source becomes production source only after confirmation.
8. Affected production data is flagged for review.

“Accept all safe mappings” applies only changes with an unambiguous scene identity. It does not silently remove existing production data.


# 162. Object Relationship Preservation Rules

Relationships survive ordinary object movement.

Moving a Scene Card between Sequences:
- keeps card identity;
- keeps comments;
- keeps attachments;
- does not create a new screenplay scene.

Duplicating a Scene Card:
- creates new identity;
- copies content;
- does not copy the source screenplay relationship.

Moving a screenplay scene by editing script order:
- changes generated scene numbering;
- keeps breakdown/shot/storyboard associations by identity where possible.

Moving a catalog item between UI groups has no effect on screenplay.


# 163. Delete Cascade Rules

|Deleted object|Default effect on children/links|User warning|
|---|---|---|
|Act|Children remain available for move/restore; no silent cascade|Yes if non-empty|
|Sequence|Scene Cards remain available for reassignment|Yes if non-empty|
|Scene Card|Screenplay scene remains|Yes if linked|
|Character|Screenplay text remains|Yes if linked|
|Catalog Item|Scene associations removed/archived only with confirmation|Yes|
|Location|Schedule history remains readable|Yes|
|Shooting Day|Call Sheet snapshots remain|Yes|
|Call Sheet|Schedule remains|Yes|
|Draft|Other drafts remain|Yes if current/locked|
|Project|Entire project may be recoverable/backup first|Strong confirmation|


# 164. Copy vs Reference Rules

The application must distinguish between:

**Copy:** independent content created from another object. Later edits do not propagate.

**Reference/association:** two objects remain distinct but one points to the other for context.

**Snapshot:** a fixed exported/issued state.

Examples:

- Global Idea → Project Idea = Copy.
- Breakdown scene → Catalog Prop = Reference/association.
- Shooting Day → Call Sheet = Source relationship + snapshot on export.
- Screenplay → Production Source = Reference to selected version/snapshot state.
- Story Board Scene → Screenplay Scene = Creation relationship, not permanent live synchronization.


# 165. Regression Matrix for Cross-Module Changes

|Change made|Verify immediately|Verify downstream|Verify must NOT change|
|---|---|---|---|
|Move Scene Card|Story Board order|Optional next build preview|Existing screenplay text|
|Edit Scene Card|Card content|Build preview if rebuilt|Existing screenplay automatically|
|Build Screenplay|New draft|Draft history|Idea Vault|
|Edit Screenplay|Current draft|Potential production stale flag|Story Board text automatically|
|Lock Draft|Locked source state|Production can select source|Older drafts|
|Script revision|New revision|Production update review|Locked source|
|Add breakdown prop|Scene breakdown|Catalog usage|Screenplay prose|
|Rename catalog prop|Catalog displays|Schedule displays|Historical PDF|
|Move schedule scene|Day assignment|Call sheet stale flag|Screenplay|
|Edit call sheet|Call sheet snapshot|None|Schedule|
|Export document|External file|None|Project content|
|Import review package|Review queue|Comments|Current draft text automatically|


# 166. Release-Blocking Functional Defects

The following defects are release blockers for the relevant release because they can cause loss of work or incorrect production documents:

- Screenplay text disappears after save/reopen.
- Draft history can be silently overwritten.
- Scene identity changes unexpectedly during ordinary reorder.
- Breakdown data disappears after screenplay revision.
- Schedule moves silently change screenplay.
- Call sheet silently overwrites an issued version.
- Exchange package import overwrites host content without confirmation.
- Offline project cannot be reopened after network loss.
- Collaborator changes are silently lost during a conflict.
- AI silently mutates project content.
- Private notes appear in external exports without explicit permission.


# 167. QA Data Sets

QA should maintain representative projects for testing:

### Small short film
5–15 scenes, 3–8 characters, 2–5 locations.

### Typical indie feature
60–120 scenes, 10–30 characters, 15–40 locations.

### Large independent project
150–250 scenes with many production elements.

### Episodic sample
1 season, 8 episodes, recurring characters and locations.

### Messy Idea Vault
Hundreds/thousands of mixed text/media/file items with intentionally incomplete metadata.

### Import stress set
FDX, PDF, Fountain, TXT, DOCX and pasted screenplay examples including malformed and unusual formatting.

These are functional data sets, not technical benchmarks.


# 168. Manual Product Walkthrough Checklist

A human reviewer should be able to perform all of the following without consulting a technical manual:

- Create project.
- Add five different Idea Vault item types.
- Switch Vault views.
- Create Act/Sequence/Beat/Scene Cards.
- Reorder cards by drag/drop.
- Move cards to Parking Lot.
- Build screenplay.
- Write scene/dialogue.
- Create named draft.
- Compare drafts.
- Add/resolve comments.
- Lock and revise.
- Import screenplay.
- Build breakdown.
- Accept/reject suggestions.
- Create locations/cast/props.
- Create moodboard/storyboard/shot list.
- Schedule scenes.
- Trigger/read conflict warning.
- Generate call sheet.
- Export documents.
- Work offline.
- Create/import exchange package.
- Conduct local-network session.
- Use AI with explicit confirmation.

A release that fails this walkthrough is not functionally complete regardless of how many individual tickets are closed.


# 169. Product Vocabulary Guardrails

Use plain wording where possible:

- “Scene Cards” instead of “Narrative Units.”
- “Story Board” instead of “Narrative Architecture Graph.”
- “Parking Lot” instead of “Inactive Branch Repository.”
- “Shooting Schedule” instead of “Production Optimization Engine.”
- “Suggest Elements” instead of “Automated Semantic Breakdown Pipeline.”
- “Review Package” instead of “Distributed Collaboration Artifact.”

Professional terminology may still appear when needed for industry documents—e.g., INT/EXT, scene heading, stripboard, call sheet—but ordinary navigation should remain approachable.


# 170. Final FSD Coverage Statement

The functional specification is intended to close the product-behavior gap between the OpenFrame Studio PRD and engineering implementation. It adds exact object behavior, field expectations, source-of-truth rules, state transitions, import/export rules, revision rules, breakdown reconciliation, scheduling behavior, call-sheet refresh behavior, offline behavior, collaboration/package behavior, AI approval behavior, destructive-action safety and regression-critical cross-module workflows.

The engineering team should use this FSD together with the PRD, not as a replacement for it. The PRD remains the authority for product purpose, target user, boundaries and priorities. The FSD is the authority for detailed expected functional behavior. A UX/UI Specification will define exact visual composition, and an ESD/technical specification will define implementation architecture and engineering mechanisms.


**Final FSD expansion size:** approximately 34,317 words / 5,345 lines.

# 171. Complete Functional Requirement Inventory

|PRD ID|ID|Requirement|Behavior contract|Priority|Verification|
|---|---|---|---|---|
|PRD-IDEA-001|FSD-IDEA-001|Create item|Add creates the selected Vault item type and places focus in the least structured useful input.|P0|User can save without metadata.|
|PRD-IDEA-001|FSD-IDEA-002|Global/project separation|Global Vault and Project Vault remain independently editable; copying is explicit.|P0|Edit project copy; global original unchanged.|
|PRD-IDEA-002|FSD-IDEA-003|Any file|Vault accepts ordinary file attachments without forcing semantic classification.|P0|Drop mixed file set and verify all valid files stored.|
|PRD-IDEA-002|FSD-IDEA-004|Untitled item|User can store an item without entering a title.|P0|Create untitled image/audio/file and reopen.|
|PRD-IDEA-003|FSD-IDEA-005|Multiple views|Grid/Card/List/Folder views present the same Vault items.|P0|Switch views and verify item count/content unchanged.|
|PRD-IDEA-003|FSD-IDEA-006|Folder move|Moving an item between folders changes organization only.|P0|Move then search globally.|
|PRD-IDEA-003|FSD-IDEA-007|Collection|Adding/removing collection membership does not duplicate or delete the item.|P0|Remove from collection; item still exists.|
|PRD-IDEA-003|FSD-IDEA-008|Pin|Pin affects display priority only.|P0|Pin and verify file path/content unchanged.|
|PRD-IDEA-004|FSD-IDEA-009|Search|Text search finds title/body/caption/tag/filename where applicable.|P0|Search exact phrase.|
|PRD-IDEA-004|FSD-IDEA-010|Preview|Supported media opens a preview without leaving project context where practical.|P0|Preview image/audio/video/PDF.|
|PRD-IDEA-002|FSD-IDEA-011|Text note autosave|Editing a note persists without requiring manual Save.|P0|Type, close, reopen.|
|PRD-IDEA-002|FSD-IDEA-012|URL fallback|URL remains usable if preview metadata cannot be retrieved.|P0|Disconnect network and open URL item.|
|PRD-IDEA-002|FSD-IDEA-013|Voice note|Recording can be saved and played later.|P0|Record/play/reopen offline.|
|PRD-IDEA-001|FSD-IDEA-014|Project copy|Copy to project creates independent content.|P0|Edit destination copy.|
|PRD-IDEA-005|FSD-IDEA-015|Move to story|Vault item can create Beat/Scene/Sequence idea/Character note/Story note.|P0|Convert each supported type.|
|PRD-IDEA-005|FSD-IDEA-016|Original retention|Moving/copying to Story never deletes the Vault original.|P0|Convert then inspect original.|
|PRD-IDEA-005|FSD-IDEA-017|No live sync|Later Story changes do not rewrite Vault item.|P0|Edit Story object and compare Vault.|
|PRD-CORE-002|FSD-IDEA-018|Soft delete|Deleted Vault items are recoverable.|P0|Delete/restore.|
|PRD-IDEA-003|FSD-IDEA-019|Multi-select|Multiple items can be selected for batch organization/deletion/export.|P0|Select five items and move to collection.|
|PRD-IDEA-004|FSD-IDEA-020|External file awareness|Referenced external files are distinguishable from project-contained files.|P0|Open external reference and inspect indicator.|
|PRD-STORY-001|FSD-STORY-001|Create Act|Act title creates a top-level story container.|P0|Add Act and inspect board/outline.|
|PRD-STORY-001|FSD-STORY-002|Create Sequence|Sequence requires only a simple name.|P0|Create “Hero Introduction”.|
|PRD-STORY-001|FSD-STORY-003|Sequence contains scenes|Scenes appear inside the sequence and preserve order.|P0|Add three scenes and reorder.|
|PRD-STORY-002|FSD-STORY-004|Create Beat|Beat is a small text card and may exist before becoming a scene.|P0|Create beat in Parking Lot.|
|PRD-STORY-003|FSD-STORY-005|Create Scene|Scene Card requires only short description; heading optional.|P0|Create card with description only.|
|PRD-STORY-003|FSD-STORY-006|Compact card|Collapsed Scene Card shows short description and minimal state indicators.|P0|Create long description and verify visual truncation only.|
|PRD-STORY-003|FSD-STORY-007|Expand card|Double click opens details drawer/editor.|P0|Open and close card.|
|PRD-STORY-004|FSD-STORY-008|Move scene|Drag moves scene within/between sequences.|P0|Drag and inspect parent/order.|
|PRD-STORY-004|FSD-STORY-009|Move sequence|Drag moves sequence between acts.|P0|Move sequence and verify children remain.|
|PRD-STORY-004|FSD-STORY-010|Move act|Drag reorders act with descendants.|P0|Reorder Act 3 before Act 2.|
|PRD-STORY-004|FSD-STORY-011|Multi-select move|Selected cards move together preserving relative order.|P0|Select three and drag.|
|PRD-STORY-004|FSD-STORY-012|Parking Lot|Card can leave active story without deletion.|P0|Park card and restore.|
|PRD-STORY-004|FSD-STORY-013|Duplicate card|Duplicate produces new independent identity.|P0|Edit duplicate and verify original.|
|PRD-STORY-004|FSD-STORY-014|No branch engine|Duplicate/History/Parking Lot support experimentation without a branching graph.|P0|Verify no branch UI appears.|
|PRD-STORY-004|FSD-STORY-015|Undo move|Story move is undoable.|P0|Move then undo.|
|PRD-STORY-004|FSD-STORY-016|Undo delete|Delete can be undone.|P0|Delete then undo.|
|PRD-STORY-001|FSD-STORY-017|Outline view parity|Board and Outline show same hierarchy/order.|P0|Reorder in each and compare.|
|PRD-STORY-001|FSD-STORY-018|Collapse act|Collapsed act hides descendants without changing data.|P0|Collapse/reopen.|
|PRD-STORY-001|FSD-STORY-019|Collapse sequence|Collapsed sequence hides children.|P0|Collapse/reopen.|
|PRD-STORY-005|FSD-STORY-020|Build screenplay selection|User can choose included active Scene Cards.|P0|Build selected subset.|
|PRD-STORY-005|FSD-STORY-021|Build preview|Preview shows cards in current order before creation.|P0|Reorder then preview.|
|PRD-STORY-005|FSD-STORY-022|Missing heading warning|Cards without heading are identified before screenplay conversion.|P0|Build with missing heading.|
|PRD-STORY-005|FSD-STORY-023|Build safety|Existing screenplay is never silently overwritten.|P0|Build when screenplay already exists.|
|PRD-STORY-005|FSD-STORY-024|Board independent after build|Editing card after build does not change script automatically.|P0|Edit card and inspect script.|
|PRD-STORY-005|FSD-STORY-025|Script order warning|Applying board reorder to a populated screenplay requires confirmation.|P0|Attempt reorder on linked script.|
|PRD-STORY-003|FSD-STORY-026|Characters optional|Scene Card creation does not require character fields.|P0|Create scene without character metadata.|
|PRD-STORY-003|FSD-STORY-027|Story day optional|Scene Card creation does not require story day.|P0|Create scene and verify no day required.|
|PRD-STORY-001|FSD-STORY-028|Sequence freeform name|No formal purpose/goal field is mandatory.|P0|Create sequence with name only.|
|PRD-STORY-002|FSD-STORY-029|Beat conversion|Beat can become a new Scene Card using its text.|P0|Convert and edit.|
|PRD-STORY-002|FSD-STORY-030|Scene-to-beat conversion|Scene card can be converted/duplicated as beat if UI supports the operation.|P0|Verify source retained.|
|PRD-SCRIPT-001|FSD-SCRIPT-001|New screenplay|User can create an empty screenplay independently of Story Board.|P0|Create screenplay and type.|
|PRD-SCRIPT-001|FSD-SCRIPT-002|Feature type|Feature screenplay project supports normal scene writing.|P0|Create feature and write.|
|PRD-SCRIPT-001|FSD-SCRIPT-003|Short type|Short film project supports same editor without extra structure.|P0|Create short and write.|
|PRD-SCRIPT-001|FSD-SCRIPT-004|Episodic type|Episode screenplay exists inside episode container.|P0|Create episode script.|
|PRD-SCRIPT-001|FSD-SCRIPT-005|Scene heading|Editor supports scene heading as first-class element.|P0|Create INT/EXT scene.|
|PRD-SCRIPT-001|FSD-SCRIPT-006|Action|Editor supports action text.|P0|Write action.|
|PRD-SCRIPT-001|FSD-SCRIPT-007|Character|Editor supports character elements.|P0|Enter character.|
|PRD-SCRIPT-001|FSD-SCRIPT-008|Dialogue|Editor supports dialogue elements.|P0|Enter dialogue.|
|PRD-SCRIPT-001|FSD-SCRIPT-009|Parenthetical|Editor supports parenthetical.|P0|Enter parenthetical.|
|PRD-SCRIPT-001|FSD-SCRIPT-010|Transition|Editor supports transitions.|P0|Enter CUT TO/other transition.|
|PRD-SCRIPT-001|FSD-SCRIPT-011|Shot direction|Optional shot direction is supported.|P0|Create shot direction.|
|PRD-SCRIPT-001|FSD-SCRIPT-012|Element switching|User can change element type manually.|P0|Convert action to dialogue.|
|PRD-SCRIPT-001|FSD-SCRIPT-013|Automatic progression|Editor suggests logical next element without blocking override.|P0|Press Enter through sample scene.|
|PRD-SCRIPT-001|FSD-SCRIPT-014|Scene navigation|Scene navigator lists current screenplay scenes.|P0|Click scene and jump.|
|PRD-SCRIPT-001|FSD-SCRIPT-015|Search|Find/search operates on current screenplay.|P0|Find phrase and next/previous.|
|PRD-SCRIPT-001|FSD-SCRIPT-016|Replace|Replace edits matching text according to selected options.|P0|Replace exact term.|
|PRD-SCRIPT-001|FSD-SCRIPT-017|Focus mode|Focus mode hides optional panels.|P0|Toggle and continue typing.|
|PRD-SCRIPT-001|FSD-SCRIPT-018|Writing room|Optional panels can open around the screenplay.|P0|Open comments/scene notes.|
|PRD-SCRIPT-001|FSD-SCRIPT-019|Idea Vault separation|Idea Vault does not automatically appear or sync in writing room.|P0|Edit Vault and verify script unaffected.|
|PRD-SCRIPT-001|FSD-SCRIPT-020|Scene notes|Private/working scene notes are excluded from standard screenplay output.|P0|Add note and export PDF.|
|PRD-SCRIPT-001|FSD-SCRIPT-021|Autosave|Typed edits persist automatically.|P0|Type/close/reopen.|
|PRD-SCRIPT-001|FSD-SCRIPT-022|Undo/redo|Text edits are undoable/redoable.|P0|Edit then undo/redo.|
|PRD-SCRIPT-002|FSD-SCRIPT-023|Named draft|User can create named draft from current/selected draft.|P0|Create Draft 2.|
|PRD-SCRIPT-002|FSD-SCRIPT-024|Draft lineage|Draft history shows parent/source lineage.|P0|Create three drafts.|
|PRD-SCRIPT-002|FSD-SCRIPT-025|Current draft|One named draft is current.|P0|Switch drafts.|
|PRD-SCRIPT-002|FSD-SCRIPT-026|Automatic history|Background recovery history exists separately from named drafts.|P0|Restore earlier edit state.|
|PRD-SCRIPT-002|FSD-SCRIPT-027|Draft comparison|Compare two drafts at scene and text level.|P0|Edit two drafts and compare.|
|PRD-SCRIPT-003|FSD-SCRIPT-028|Review round|Create review linked to selected draft.|P0|Create review.|
|PRD-SCRIPT-003|FSD-SCRIPT-029|Comment text|Comment can anchor to selected script text.|P0|Select text and comment.|
|PRD-SCRIPT-003|FSD-SCRIPT-030|Comment scene|General comment can attach to scene.|P0|Create scene comment.|
|PRD-SCRIPT-003|FSD-SCRIPT-031|Reply|User can reply within thread.|P0|Reply and inspect thread.|
|PRD-SCRIPT-003|FSD-SCRIPT-032|Resolve|Comment can be marked resolved but retained in history.|P0|Resolve and reopen review.|
|PRD-SCRIPT-003|FSD-SCRIPT-033|Private note|Private note is not visible to unauthorized users.|P0|Create private note and export.|
|PRD-SCRIPT-004|FSD-SCRIPT-034|Lock draft|Selected draft can become Shooting Draft.|P1|Lock and inspect status.|
|PRD-SCRIPT-004|FSD-SCRIPT-035|Locked edit gate|Editing locked draft starts revision flow.|P1|Attempt edit after lock.|
|PRD-SCRIPT-004|FSD-SCRIPT-036|Revision label|Production revision can have label/color.|P1|Create Revision A/Blue.|
|PRD-SCRIPT-004|FSD-SCRIPT-037|Revision comparison|Production revision shows changed scenes/content.|P1|Edit and inspect changes.|
|PRD-SCRIPT-005|FSD-SCRIPT-038|Import FDX|FDX can be imported as screenplay/draft.|P0|Import known FDX.|
|PRD-SCRIPT-005|FSD-SCRIPT-039|Import PDF|PDF import provides preview/warnings.|P0|Import formatted PDF.|
|PRD-SCRIPT-005|FSD-SCRIPT-040|Import Fountain|Fountain imports structure.|P0|Import Fountain.|
|PRD-SCRIPT-005|FSD-SCRIPT-041|Import TXT|TXT becomes parsed screenplay where possible.|P0|Import TXT.|
|PRD-SCRIPT-005|FSD-SCRIPT-042|Import DOCX|DOCX text/structure is parsed where recognizable.|P0|Import DOCX.|
|PRD-SCRIPT-005|FSD-SCRIPT-043|Paste screenplay|Pasted text can be parsed.|P0|Paste sample.|
|PRD-SCRIPT-005|FSD-SCRIPT-044|Import safety|Import does not overwrite current draft by default.|P0|Import into populated project.|
|PRD-SCRIPT-006|FSD-SCRIPT-045|Export PDF|Current draft exports to professional PDF.|P0|Export and inspect.|
|PRD-SCRIPT-006|FSD-SCRIPT-046|Export FDX|Current draft exports FDX.|P0|Export/reimport.|
|PRD-SCRIPT-006|FSD-SCRIPT-047|Export Fountain|Current draft exports Fountain.|P0|Export/reimport.|
|PRD-SCRIPT-006|FSD-SCRIPT-048|Export DOCX|Current draft exports editable DOCX.|P0|Export/reopen.|
|PRD-SCRIPT-006|FSD-SCRIPT-049|Title page|Screenplay export supports title-page data.|P0|Set title/author and export.|
|PRD-SCRIPT-006|FSD-SCRIPT-050|Scene numbering|Display numbers derive from screenplay order.|P0|Reorder scenes and verify numbers.|
|PRD-BRK-001|FSD-BREAKDOWN-001|Select source|Breakdown requires selected screenplay source.|P0|Choose draft.|
|PRD-BRK-001|FSD-BREAKDOWN-002|Scene list|All source scenes appear in breakdown.|P0|Inspect count/headings.|
|PRD-BRK-001|FSD-BREAKDOWN-003|Script pane|Scene script remains visible while tagging.|P0|Open scene and tag.|
|PRD-BRK-001|FSD-BREAKDOWN-004|Manual tag|User can manually add category/item.|P0|Add pistol prop.|
|PRD-BRK-001|FSD-BREAKDOWN-005|Highlight tag|User can select matching text before tagging.|P0|Select word and tag.|
|PRD-BRK-001|FSD-BREAKDOWN-006|Categories|Default categories remain limited to defined list.|P0|Verify category list.|
|PRD-BRK-001|FSD-BREAKDOWN-007|Suggest elements|System may propose likely breakdown elements.|P0|Run suggestions.|
|PRD-BRK-001|FSD-BREAKDOWN-008|Accept suggestion|Accepted suggestion becomes production data.|P0|Accept one.|
|PRD-BRK-001|FSD-BREAKDOWN-009|Reject suggestion|Rejected suggestion does not create production data.|P0|Reject one.|
|PRD-BRK-001|FSD-BREAKDOWN-010|Edit suggestion|User can modify category/name before accepting.|P0|Edit and accept.|
|PRD-BRK-001|FSD-BREAKDOWN-011|Batch suggestion accept|User can accept several suggestions after review.|P0|Accept selected batch.|
|PRD-BRK-001|FSD-BREAKDOWN-012|Catalog reuse|Existing catalog item can be attached to scene.|P0|Attach existing prop.|
|PRD-BRK-001|FSD-BREAKDOWN-013|New catalog item|User can create catalog item from breakdown.|P0|Create new prop.|
|PRD-BRK-001|FSD-BREAKDOWN-014|Remove scene association|Removing breakdown element leaves catalog item intact.|P0|Remove and inspect catalog.|
|PRD-BRK-001|FSD-BREAKDOWN-015|Breakdown complete|Scene may be marked complete manually.|P0|Mark and filter.|
|PRD-BRK-001|FSD-BREAKDOWN-016|Needs review|Changed production source flags affected scenes.|P0|Update source and inspect.|
|PRD-BRK-001|FSD-BREAKDOWN-017|No silent deletion|Script change never silently deletes breakdown data.|P0|Remove script scene and update.|
|PRD-BRK-001|FSD-BREAKDOWN-018|New scene state|New script scene enters Needs Breakdown.|P0|Add scene and update source.|
|PRD-BRK-001|FSD-BREAKDOWN-019|Changed heading|Heading change flags source review.|P0|Edit location heading.|
|PRD-PROD-001|FSD-PROD-001|Catalog|Production catalog exists for reusable production items.|P0|Create and search item.|
|PRD-PROD-001|FSD-PROD-002|Catalog usage|Item shows scenes using it.|P0|Assign prop to two scenes.|
|PRD-PROD-001|FSD-PROD-003|Catalog status|Item can have simple status.|P0|Change status.|
|PRD-LOC-001|FSD-PROD-004|Location create|Location requires name only initially.|P0|Create location.|
|PRD-LOC-001|FSD-PROD-005|Location photos|Photos can be attached.|P0|Add photos.|
|PRD-LOC-001|FSD-PROD-006|Location notes|Practical notes available.|P0|Add parking/noise notes.|
|PRD-LOC-001|FSD-PROD-007|Location scenes|Location shows linked scenes.|P0|Assign and inspect.|
|PRD-CAST-001|FSD-PROD-008|Cast create|Actor can be created with character association.|P0|Add actor.|
|PRD-CAST-001|FSD-PROD-009|Crew create|Crew member can be created with role/department.|P0|Add crew.|
|PRD-CAST-001|FSD-PROD-010|Cast availability|Availability can be recorded as notes.|P0|Enter availability.|
|PRD-VIS-001|FSD-PROD-011|Moodboard create|Visual board can be created and renamed.|P0|Create board.|
|PRD-VIS-001|FSD-PROD-012|Moodboard item|Image/note/link can be added and moved.|P0|Add and drag.|
|PRD-VIS-001|FSD-PROD-013|Moodboard export|Moodboard can export cleanly.|P0|Export PDF.|
|PRD-STB-001|FSD-PROD-014|Storyboard create|Storyboard can be created for scene.|P0|Create board.|
|PRD-STB-001|FSD-PROD-015|Storyboard panel|Panel has visual area and short description.|P0|Create panel.|
|PRD-STB-001|FSD-PROD-016|Storyboard reorder|Panels reorder by drag.|P0|Move panel.|
|PRD-STB-001|FSD-PROD-017|Storyboard shot link|Panel can link to shot.|P0|Attach panel to shot.|
|PRD-SHOT-001|FSD-PROD-018|Shot create|Shot can be created with description only.|P0|Add shot.|
|PRD-SHOT-001|FSD-PROD-019|Shot reorder|Shots reorder by drag.|P0|Move shot.|
|PRD-SHOT-001|FSD-PROD-020|Shot optional fields|Camera/technical fields remain optional.|P0|Create shot without lens.|
|PRD-SHOT-001|FSD-PROD-021|Shot export|Shot list exports clearly.|P0|Export PDF.|
|PRD-PROD-002|FSD-PROD-022|Daily view|Shooting day shows scenes/cast/locations/estimates.|P1|Open day.|
|PRD-PROD-003|FSD-PROD-023|Reports|Basic practical reports available.|P1|Generate scene/location/prop report.|
|PRD-PROD-004|FSD-PROD-024|Sides|Selected shooting day can create sides.|P1|Generate PDF.|
|PRD-PROD-005|FSD-PROD-025|Budget snapshot|Simple budget categories/line items can be entered.|P1|Add lines and verify total.|
|PRD-PROD-005|FSD-PROD-026|Budget boundary|Budget does not implement payroll/accounting.|P1|Verify no payroll workflow.|
|PRD-SCHED-001|FSD-SCHED-001|Create schedule|Schedule is created from selected screenplay source.|P0|Create schedule.|
|PRD-SCHED-001|FSD-SCHED-002|Unscheduled pool|All scenes initially appear unscheduled.|P0|Inspect pool.|
|PRD-SCHED-001|FSD-SCHED-003|Shooting day|Date/day can be created.|P0|Create day.|
|PRD-SCHED-001|FSD-SCHED-004|Drag scene|Scene moves to day via drag.|P0|Drag and inspect.|
|PRD-SCHED-001|FSD-SCHED-005|Reorder day|Scenes reorder within day.|P0|Drag within day.|
|PRD-SCHED-001|FSD-SCHED-006|Move day|Scene can move between days.|P0|Move scene.|
|PRD-SCHED-001|FSD-SCHED-007|Off day|Off day can exist without scenes.|P0|Create off day.|
|PRD-SCHED-001|FSD-SCHED-008|Break marker|Meal/travel/company move/custom marker can be added.|P0|Add break.|
|PRD-SCHED-001|FSD-SCHED-009|Duration|Day can display estimated total.|P0|Enter durations.|
|PRD-SCHED-002|FSD-SCHED-010|Conflict actor|Potential actor conflict is warned.|P1|Create overlap.|
|PRD-SCHED-002|FSD-SCHED-011|Conflict location|Potential location conflict is warned.|P1|Schedule conflict.|
|PRD-SCHED-002|FSD-SCHED-012|Overflow|Duration overflow is warned.|P1|Exceed day target.|
|PRD-SCHED-002|FSD-SCHED-013|Keep anyway|Warning can be overridden.|P1|Keep conflicting schedule.|
|PRD-SCHED-001|FSD-SCHED-014|Suggestions|System can suggest grouping scenes but not silently move.|P0|Request suggestion.|
|PRD-SCHED-001|FSD-SCHED-015|Calendar|Dates map to shooting days.|P0|Change date.|
|PRD-SCHED-001|FSD-SCHED-016|Schedule export|Schedule exports snapshot.|P0|Export PDF.|
|PRD-CALL-001|FSD-CALL-001|Create from day|Call sheet can be created from shooting day.|P0|Create from Day 1.|
|PRD-CALL-001|FSD-CALL-002|Prefill scenes|Scheduled scenes populate automatically.|P0|Generate.|
|PRD-CALL-001|FSD-CALL-003|Prefill cast|Cast data populates automatically where available.|P0|Generate.|
|PRD-CALL-001|FSD-CALL-004|Prefill location|Location/address populates automatically where available.|P0|Generate.|
|PRD-CALL-001|FSD-CALL-005|Call time edit|User can edit call times.|P0|Edit and save.|
|PRD-CALL-001|FSD-CALL-006|Day notes|User can add day-specific notes.|P0|Add note.|
|PRD-CALL-001|FSD-CALL-007|Optional sections|Weather/attachments/special notes are optional.|P0|Add/remove.|
|PRD-CALL-001|FSD-CALL-008|No reverse sync|Call-sheet edit does not rewrite schedule.|P0|Edit call sheet and inspect schedule.|
|PRD-CALL-001|FSD-CALL-009|Stale state|Source schedule changes mark call sheet as stale.|P0|Change schedule.|
|PRD-CALL-001|FSD-CALL-010|Refresh preview|Refreshing shows what will change before applying.|P0|Refresh and inspect.|
|PRD-CALL-001|FSD-CALL-011|Issued snapshot|Issued/exported call sheet remains historical.|P0|Export then change schedule.|
|PRD-CALL-001|FSD-CALL-012|PDF export|Call sheet produces professional PDF.|P0|Export.|
|PRD-COL-001|FSD-COL-001|Owner|Owner has full project control.|P1|Verify permissions.|
|PRD-COL-001|FSD-COL-002|Editor|Editor can edit authorized content.|P1|Edit as contributor.|
|PRD-COL-001|FSD-COL-003|Commenter|Commenter can review/comment but not edit core content.|P1|Attempt edit.|
|PRD-COL-001|FSD-COL-004|Viewer|Viewer is read-only.|P1|Attempt modification.|
|PRD-COL-001|FSD-COL-005|Export-only|Export-only can create permitted snapshot/package.|P1|Export and attempt edit.|
|PRD-COL-003|FSD-COL-006|Exchange story|Story Exchange package exists.|P1|Export/import.|
|PRD-COL-002|FSD-COL-007|Exchange script|Script Review package exists.|P0|Export/import.|
|PRD-COL-003|FSD-COL-008|Exchange breakdown|Breakdown exchange exists.|P1|Export/import.|
|PRD-COL-003|FSD-COL-009|Exchange shots|Shot exchange exists.|P1|Export/import.|
|PRD-COL-003|FSD-COL-010|Exchange schedule|Schedule exchange exists.|P1|Export/import.|
|PRD-COL-003|FSD-COL-011|Exchange call|Call-sheet review package exists.|P1|Export/import.|
|PRD-COL-004|FSD-COL-012|Preview before apply|Package import shows changes before mutation.|P1|Import and cancel.|
|PRD-COL-004|FSD-COL-013|Stale review|Older review is recognized as stale.|P1|Import response from old draft.|
|PRD-COL-004|FSD-COL-014|No blind overwrite|Import never silently destroys host data.|P1|Import conflicting package.|
|PRD-COL-004|FSD-COL-015|Ambiguous mapping|Ambiguous comments enter review queue.|P1|Create duplicate text context.|
|PRD-COL-004|FSD-COL-016|Unmapped note|Unmapped comment is retained.|P1|Force no match.|
|PRD-COL-005|FSD-COL-017|Local host|LAN host can start session.|P0|Start session.|
|PRD-COL-005|FSD-COL-018|Join|Participant can join with granted role.|P0|Join.|
|PRD-COL-005|FSD-COL-019|Presence|Participant presence is visible where useful.|P0|Two-user session.|
|PRD-COL-006|FSD-COL-020|Same-object conflict|Conflicting edits surface for review.|P1|Edit same text concurrently.|
|PRD-COL-005|FSD-COL-021|Disconnect|Local project remains usable after participant disconnect.|P0|Disconnect test.|
|PRD-COL-005|FSD-COL-022|End session|Ending session does not publish project to cloud.|P0|End and reopen local.|
|PRD-OFF-001|FSD-OFF-001|Offline open|Project opens without network.|P0|Disable network/open.|
|PRD-OFF-001|FSD-OFF-002|Offline edit|Core editing works offline.|P0|Edit Vault/story/script.|
|PRD-OFF-001|FSD-OFF-003|Offline production|Breakdown/schedule/call sheet work offline.|P0|Exercise production offline.|
|PRD-OFF-001|FSD-OFF-004|Offline export|Documents export offline.|P0|Export PDF.|
|PRD-OFF-001|FSD-OFF-005|Save status|Status reflects local persistence.|P0|Save offline.|
|PRD-OFF-001|FSD-OFF-006|Recovery|Crash recovery restores latest safe state.|P0|Simulate interruption.|
|PRD-OFF-001|FSD-OFF-007|Backup|User can create backup package.|P0|Create backup.|
|PRD-OFF-001|FSD-OFF-008|Full project portability|Project can move to another machine.|P0|Export/import package.|
|PRD-OFF-001|FSD-OFF-009|External drive|Project on external drive works while mounted.|P0|Open project from drive.|
|PRD-OFF-001|FSD-OFF-010|Drive loss|Drive removal does not silently create divergent project.|P0|Remove drive and inspect warning.|
|PRD-AI-001|FSD-AI-001|Optional|AI disabled does not disable core app.|P1|Run without AI.|
|PRD-AI-001|FSD-AI-002|Context selector|User can choose current scene/screenplay/story/project/production context.|P1|Change scope.|
|PRD-AI-001|FSD-AI-003|Question answering|AI can answer content questions without changing data.|P1|Ask summary question.|
|PRD-AI-001|FSD-AI-004|Scene card suggestion|AI can propose a Scene Card.|P1|Ask conversion.|
|PRD-AI-001|FSD-AI-005|Breakdown suggestion|AI can suggest likely elements.|P1|Run on scene.|
|PRD-AI-001|FSD-AI-006|Synopsis|AI can create synopsis from selected content.|P1|Generate synopsis.|
|PRD-AI-001|FSD-AI-007|Schedule advice|AI can explain grouping opportunities.|P1|Ask for schedule advice.|
|PRD-AI-001|FSD-AI-008|Mutation preview|Any project-changing AI action shows preview.|P1|Ask AI to create item.|
|PRD-AI-001|FSD-AI-009|Accept/reject|User controls mutation.|P1|Reject suggestion and verify no change.|
|PRD-AI-001|FSD-AI-010|No silent rewrite|AI does not rewrite screenplay silently.|P1|Ask for rewrite and inspect approval step.|
|PRD-AI-001|FSD-AI-011|External disclosure|Cloud AI use is clearly disclosed.|P1|Invoke external AI.|
|PRD-AI-001|FSD-AI-012|AI failure safety|AI failure leaves local data unchanged.|P1|Disconnect and invoke AI.|
|PRD-AI-001|FSD-AI-013|System-wide natural-language command interface|User can issue supported navigation and application requests in ordinary language.|P1|Invoke supported commands from shell/context.|
|PRD-AI-001|FSD-AI-014|Deterministic project statistics and exact queries|Supported exact counts and metrics come from canonical project queries/calculations.|P1|Ask known count and compare with source data.|
|PRD-AI-001|FSD-AI-015|Cross-module relationship queries|AI can retrieve supported relationships across story, screenplay and production.|P1|Ask cross-module relationship question.|
|PRD-AI-001|FSD-AI-016|Natural-language navigation and application routing|Supported read/navigation requests route to existing application actions.|P1|Ask AI to open a workspace/object.|
|PRD-AI-001|FSD-AI-017|Structured project-wide rename/replace|Rename distinguishes structured references from arbitrary screenplay text.|P1|Prepare character rename and inspect categories.|
|PRD-AI-001|FSD-AI-018|Batch mutation preparation and review|Batch proposals show affected counts, exclusions and conflicts before application.|P1|Prepare multi-object mutation and review.|
|PRD-AI-001|FSD-AI-019|Permission inheritance|AI cannot exceed current user's project/object permissions.|P1|Attempt mutation as Viewer/Commenter.|
|PRD-AI-001|FSD-AI-020|Locked/private/approval state enforcement|AI respects locked content, private notes and approval gates.|P1|Attempt protected action.|
|PRD-AI-001|FSD-AI-021|Change Set base-version validation|Stale proposals require revalidation before application.|P1|Change project after proposal and reapply.|
|PRD-AI-001|FSD-AI-022|Application tool boundary and validation|AI cannot directly write project storage and tool calls are validated.|P1|Inspect denied/bypassed write attempt.|
|PRD-AI-001|FSD-AI-023|Product knowledge access|AI can answer supported questions about OpenFrame behavior from current product knowledge.|P1|Ask workflow/rule question.|
|PRD-AI-001|FSD-AI-024|Scope/provenance disclosure|Materially scoped answers identify relevant scope/source context.|P1|Ask same metric under different scopes.|
|PRD-AI-001|FSD-AI-025|AI lifecycle/activity integration|Accepted AI mutations are recorded as normal application history/activity.|P1|Accept mutation and inspect history/undo.|
|PRD-AI-001|FSD-AI-026|Conversation/session context|Follow-up requests can reuse safe session context without mutating data.|P1|Issue contextual follow-up request.|
|PRD-AI-001|FSD-AI-027|Cross-project/episodic scope control|AI preserves project boundaries and explicit episode/series scope.|P1|Ask cross-episode/project query.|

# 172. Field Validation and Input Policy Matrix

|Object|Field|Requiredness|Validation intent|Used in|
|---|---|---|---|---|
|Project|Title|Required|1+ readable characters|Create/open/home|
|Project|Type|Required|Feature / Short / Episodic|Workflow|
|Project|Language|Optional|Free text|Project metadata|
|Project|Genre|Optional|Free text / selected tags|Project metadata|
|Project|Status|Optional|Defined lifecycle labels|Home|
|Idea|Type|Required|Supported item type|Rendering|
|Idea|Title|Optional|May be blank|Vault display|
|Idea|Body/Caption|Optional|Plain text|Preview/search|
|Act|Title|Required|Non-empty|Story Board|
|Sequence|Name|Required|Non-empty|Story Board grouping|
|Beat|Text|Required for useful beat|Non-empty after save unless intentionally parked|Story Board|
|Scene Card|Description|Required for active scene|Text|Story Board/screenplay build|
|Scene Card|Heading|Optional|Valid screenplay heading when used|Build Screenplay|
|Character|Name|Required|Non-empty|Character directory|
|Draft|Name|Required|Non-empty|Draft history|
|Review|Source Draft|Required|Existing draft|Review workflow|
|Breakdown Element|Category|Required|Default category|Breakdown|
|Breakdown Element|Name|Required|Non-empty|Catalog|
|Catalog Item|Name|Required|Non-empty|Production|
|Catalog Item|Status|Required|Allowed status|Production|
|Location|Name|Required|Non-empty|Production|
|Person|Name|Required|Non-empty|Cast/Crew|
|Person|Role/Department|Required for crew|Non-empty|Crew|
|Shot|Description|Required|Text|Shot list|
|Storyboard Panel|Description|Optional|Text|Storyboard|
|Shooting Day|Date|Required|Valid date|Schedule|
|Shooting Day|Day Number|Required|Positive integer/display order|Schedule|
|Call Sheet|Source Day|Required|Existing shooting day|Call sheet|
|Task|Title|Required|Non-empty|Project task|
|Budget Line|Description|Required|Text|Budget|
|Budget Line|Amount|Required|Non-negative monetary value|Budget total|


# 173. Cross-Module Event Contract

|ID|Event|Required result|Primary area|
|---|---|---|---|
|EVT-001|Scene Card moved|Story Board order/parent changes; undo entry created; screenplay unchanged automatically.|Story Board|
|EVT-002|Scene Card duplicated|New card identity created with copied content; screenplay relationship absent.|Story Board|
|EVT-003|Scene Card parked|Active outline excludes card; card retained in Parking Lot.|Story Board|
|EVT-004|Scene Card restored|Card placed at user-selected insertion point; identity retained.|Story Board|
|EVT-005|Build Screenplay confirmed|New screenplay/draft created from selected cards; existing drafts untouched.|Story → Screenplay|
|EVT-006|Existing script reorder request|Warning displayed before applying board order to written screenplay.|Story/Screenplay|
|EVT-007|Screenplay draft created|Parent draft remains unchanged; new current/non-current state applied per selection.|Screenplay|
|EVT-008|Screenplay locked|Selected draft becomes shooting baseline; editing prompts revision.|Screenplay|
|EVT-009|Production source updated|Scene diff shown; affected production objects flagged, not deleted.|Screenplay → Production|
|EVT-010|Scene removed from source|Production records retain historical mapping and are flagged.|Breakdown/Schedule|
|EVT-011|Scene added to source|New scene enters production review/unscheduled state.|Breakdown/Schedule|
|EVT-012|Catalog item attached|Scene gains association; catalog identity reusable.|Breakdown/Catalog|
|EVT-013|Catalog item renamed|Active production references display new name; historical exports remain unchanged.|Catalog|
|EVT-014|Schedule scene moved|Day totals update; affected call sheet becomes stale if generated.|Schedule/Call Sheet|
|EVT-015|Schedule date changed|Call sheet becomes stale if sourced from affected day.|Schedule/Call Sheet|
|EVT-016|Call sheet edited|Call sheet changes only; schedule unchanged.|Call Sheet|
|EVT-017|Call sheet refreshed|Preview changes then new snapshot state saved.|Call Sheet|
|EVT-018|Export complete|Snapshot created; project not mutated.|Export|
|EVT-019|Exchange imported|Preview then selected changes/comments added; no blind overwrite.|Collaboration|
|EVT-020|AI mutation accepted|User-confirmed change becomes normal undoable project action.|AI|


# 174. Canonical End-to-End Scenario Catalog

|ID|Scenario|Given|Action|Expected result|
|---|---|---|---|---|
|SC-001|New project from scratch|No project open|Create project and enter title/type|New project opens at Project Home with empty workspaces and three clear next actions.|
|SC-002|Global idea copied to project|Global Vault contains item|Copy item to Project A|Project copy is independent and appears in Project Vault.|
|SC-003|Messy idea collection|Project Vault has mixed files|Add 10 items without tags or titles|All items remain stored and searchable without classification.|
|SC-004|Card outline|Empty Story Board|Create Act, Sequence and five Scene Cards|Cards are compact, ordered and editable through drag/drop.|
|SC-005|Short film no sequences|Short project|Create Act and direct Scene Cards|No sequence is required; screenplay build remains possible.|
|SC-006|Parking Lot experiment|Board has scenes|Park two cards and restore one|Active story excludes parked cards; restored card returns to outline.|
|SC-007|Build screenplay|Board has valid headings|Build screenplay|New screenplay contains scenes in card order with generated numbers.|
|SC-008|Build with missing headings|Board has one card without heading|Build screenplay|User is prompted to supply heading or exclude the card.|
|SC-009|Write without board|New project|Create screenplay directly|User can write normally; Story Board remains optional.|
|SC-010|Import existing script|Project has no script|Import FDX|Draft appears with parsed scenes and import summary.|
|SC-011|Draft history|Draft 1 exists|Create Draft 2, edit, compare|Draft 1 is unchanged and comparison shows differences.|
|SC-012|Review round|Draft 3 exists|Create review and comments|Comments remain attached and resolvable.|
|SC-013|Lock/revision|Draft 5 exists|Lock, attempt edit, create revision|Locked source remains intact; revision is separate.|
|SC-014|Breakdown|Shooting Draft exists|Open Breakdown|All screenplay scenes appear with source label.|
|SC-015|Breakdown suggestions|Scene contains obvious prop/location/cast|Suggest and accept/reject|Only accepted items become production data.|
|SC-016|Catalog reuse|Prop exists in Catalog|Tag same prop in second scene|Second scene references same catalog item.|
|SC-017|Location planning|Location exists|Add photos and notes and confirm|Location shows scenes and practical notes.|
|SC-018|Visual planning|Scene exists|Create storyboard and shot list|Both reference same scene without requiring each other.|
|SC-019|Schedule|Script and production data exist|Create shooting days and drag scenes|Scene assignment and day totals update.|
|SC-020|Schedule conflict|Actor appears in two locations|Schedule both scenes with overlap|Advisory warning appears; user can keep schedule.|
|SC-021|Call sheet|Shoot Day 4 scheduled|Create call sheet, add call times, export|Call sheet contains day data and produces PDF snapshot.|
|SC-022|Schedule revision after call sheet|Issued call sheet exists|Move scene to another day|Issued file remains; call sheet source shows stale/refresh state.|
|SC-023|Offline project|Network disabled|Open/edit/export project|Core operations continue locally.|
|SC-024|Full project move|Project complete|Export package and import to second machine|Project opens as working copy with supported content.|
|SC-025|Remote review|Reviewer is on another machine|Export review package; reviewer comments; response imported|Comments map or enter review queue; no blind overwrite.|
|SC-026|LAN collaboration|Two machines on same private network|Host/join/edit|Changes appear to participants and session can end without cloud.|
|SC-027|AI breakdown|AI configured|Ask for suggestions and accept two|Only accepted suggestions enter breakdown.|
|SC-028|AI unavailable|No network/AI|Ask AI|AI action fails clearly; local project unchanged.|
|SC-029|Delete linked scene card|Scene card created screenplay scene|Delete card|Screenplay scene remains and can be found.|
|SC-030|Restore deleted item|Deleted location|Restore|Location returns or enters unassigned state if original container unavailable.|


# 175. Canonical User Journey — Day 1 to Shooting Day

### 175.1 Day 1: raw idea
The user opens OpenFrame, creates a project, and immediately enters Project Idea Vault. They can add text, screenshots, a video reference, a PDF research document, a voice note and a handwritten sketch without entering a title or classification. They close the app.

**Expected state on reopen:** all items are present, the project opens at the last meaningful location, and no setup wizard is required.

### 175.2 Story shaping
The user creates Act 1, then a Sequence named `Hero Introduction`. They create six Scene Cards using only short descriptions. They drag cards until the sequence feels correct. They park two unused ideas.

**Expected state:** Story Board contains a usable outline with no manual scene numbering and no character/production metadata.

### 175.3 Screenplay creation
User adds headings to the scenes selected for the screenplay and clicks Build Screenplay. OpenFrame previews the order and confirms a new draft. The screenplay opens with the scenes in order.

**Expected state:** screenplay has generated scene numbers; Story Board remains an independent reference.

### 175.4 Writing
The user writes action/dialogue for several weeks, creates named drafts, and conducts a review round. Comments are attached to text and scenes. Resolved comments remain in history.

### 175.5 Lock
User locks the chosen draft. Any subsequent edit starts a revision. A revision label/color can be assigned.

### 175.6 Production planning
User chooses the locked/revised screenplay as Production Source. Breakdown scenes appear. User accepts only useful automatic suggestions. Confirmed items populate Catalog. User adds locations and cast.

### 175.7 Visual planning
User creates moodboards and a storyboard for the main action scene, then builds a practical shot list. The same scene identity is retained through all views.

### 175.8 Schedule
User creates shooting days and drags scenes onto them. The app warns about actor/location conflicts and duration overload but never forces a change.

### 175.9 Call sheet
User selects a shooting day, generates a call sheet, enters call times and parking instructions, exports PDF and sends it through their own communication channel.

### 175.10 Result
The filmmaker reaches the shoot with the same project information used to develop and write the movie. The software has reduced duplicate work without attempting to manage unrelated studio operations.


# 176. Functional Test Oracle — What “Correct” Looks Like

A tester should judge results by the following principles:

**Correctness:** The requested object/action occurs.

**Persistence:** Reopening the project retains the action.

**Identity:** Moving an object does not unexpectedly create a different identity.

**Safety:** Destructive/ambiguous changes do not silently destroy data.

**Separation:** A module does not automatically mutate another module unless a documented connection requires it.

**Traceability:** Production data can identify its screenplay source where applicable.

**Export fidelity:** Exported documents contain the selected source content and exclude private/internal information by default.

**Offline ownership:** Core workflows remain available without internet.

**Human control:** Suggestions are not decisions.


# 177. Final Implementation Questions That Must Be Resolved Before Coding

The following are the only categories of product questions that may legitimately remain for the UX/ESD companion specifications. They should not be silently invented by individual engineers:

1. Exact visual dimensions and responsive layout of desktop panels.
2. Exact keyboard shortcut assignments where multiple conventions exist.
3. Exact visual design of revision colors and status badges.
4. Exact project package file extension/naming convention.
5. Exact exchange package file extensions/naming convention.
6. Exact offline project storage implementation.
7. Exact local-network session transport/hosting implementation.
8. Exact external AI provider integration method.
9. Exact FDX/PDF/DOCX parsing libraries/engines.
10. Exact PDF/document rendering implementation.

Everything else required for user-visible product behavior should be governed by the PRD + FSD + UX/UI Specification. When engineering encounters a case outside those documents, it should log a specification question rather than inventing a behavior that changes product semantics.


**FSD expanded coverage size:** approximately 39,128 words / 5,749 lines.