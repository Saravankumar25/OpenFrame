# OpenFrame Studio — UX/UI Specification

## Desktop Edition — Product / Functional Alignment Baseline

**Document type:** UX/UI Specification Document
**Product:** OpenFrame Studio
**Primary platform:** Windows + macOS desktop application
**Operating model:** Local-first, offline-capable, user-owned project files; optional internet-assisted AI; optional local-network collaboration; no mandatory OpenFrame cloud.
**Target users:** Independent filmmakers, writer-directors, small production teams, student filmmakers, short-film makers, regional-language filmmakers, and small episodic teams.
**Source documents:**
- `OpenFrame_Studio_Mega_PRD_Aligned_Updated.md`
- `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md`
- `OpenFrame_Studio_AI_Specification_Updated.md`
**Generated:** 2026-09-29
**Status:** Design-definition baseline for UX, visual design, product review, and engineering handoff.

> This document defines the observable user interface and interaction behavior required to present the product defined by the PRD and FSD. It does not introduce a new product scope, technical architecture, database model, API, cloud architecture, or implementation stack.

# 0. Purpose and Design Contract
This document translates the OpenFrame Studio Product Requirements Document and Functional Specification Document into a concrete desktop user experience.

The PRD establishes the product identity, scope, priorities, target audience, and product philosophy. The FSD establishes functional behavior, object behavior, state transitions, source-of-truth rules, import/export behavior, offline behavior, collaboration behavior, acceptance scenarios, and field-level rules. This document sits beneath those two documents and describes how that behavior is presented and operated by a filmmaker.

The UX/UI specification must therefore do three things at once:

1. Make the product feel simple enough for a first-time filmmaker.
2. Expose enough professional controls for a real independent production.
3. Never turn the product into an enterprise production-management interface.

The governing UX concept is the same one established by the product documents:

> **One film, progressively structured: messy ideas → story outline → screenplay → production plan.**

The user should be able to move between workspaces without feeling that they have entered completely different software. At the same time, the user should not be forced to see every relationship everywhere. A Scene may connect to screenplay, breakdown, shots, storyboard, schedule and call sheet, but those relationships appear through focused navigation and compact contextual views rather than one giant all-purpose screen.

Where the FSD specifies exact behavior, this document preserves it. Where the FSD intentionally leaves visual presentation flexible, the design decisions below choose the simplest desktop interaction consistent with the product philosophy.

## 0.1 Authority hierarchy
The documents have a strict hierarchy:

**PRD → FSD → UX/UI Specification → implementation**

The PRD owns product intent, scope and priority. The FSD owns functional behavior. This UX/UI document owns presentation, interaction, navigation and user-facing states. The UX/UI layer must not change product scope.

If the UX designer discovers that a proposed interaction requires a new product capability that is not represented by a PRD requirement or FSD requirement, the feature must be treated as a specification gap rather than silently becoming part of the product.

## 0.2 Alignment rule
Every screen and interaction in this document must map to one or more approved PRD/FSD requirements.

No standalone notification center, enterprise administration center, accounting system, payroll system, VFX system, post-production system, distribution CRM, cloud media platform, mobile product, or other deferred capability may appear as an independent product surface.

Contextual status indicators are allowed where the FSD requires them, for example a stale production-source banner or a call sheet marked as needing refresh. These are status communication elements, not a separate notification product.

## 0.3 Plain-language design
OpenFrame should use ordinary language wherever possible:

- **Story Board** instead of structural-analysis terminology.
- **Scene Cards** instead of abstract story-unit terminology.
- **Shooting Schedule** alongside Stripboard where professional language is useful.
- **Project Files** instead of asset-repository terminology.
- **Review** instead of collaboration workflow jargon.

Professional terms may still appear where they are standard and useful, but the application should never require the user to understand film-school terminology before they can perform the basic action.

## 0.4 Desktop-first assumptions
The design is for a resizable Windows/macOS desktop application. The primary interaction model is mouse + keyboard, with trackpad support on macOS and common precision pointing on Windows.

The app should work comfortably in a large maximized window, remain usable at narrower desktop widths, support fullscreen writing mode, and leave room for optional secondary windows/panels because the PRD explicitly recognizes multi-monitor workflows as a desktop advantage.

Mobile screens are not treated as primary UI surfaces. Exported PDFs should remain readable on smaller devices, but the product itself remains desktop-first.

# 1. Global UX Architecture
The application is organized around a stable shell and a changing workspace. The shell should remain familiar while the center of the application changes from ideas to story to screenplay to production.

The product should feel visually related to a professional creative tool rather than a business dashboard.

## 1.1 Main application shell
Recommended desktop composition:

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ OpenFrame   Project Name ▾     Global Search     Undo Redo   Status   User │
├──────────────────────┬─────────────────────────────────────────────────────┤
│ Home                 │                                                     │
│ Idea Vault           │                                                     │
│ Story                │                    ACTIVE WORKSPACE                 │
│ Screenplay           │                                                     │
│ Breakdown            │                                                     │
│ Production           │                                                     │
│ Call Sheets          │                                                     │
│ Files                │                                                     │
├──────────────────────┴─────────────────────────────────────────────────────┤
│ Saved / Saving / Offline / Collaboration / contextual status               │
└────────────────────────────────────────────────────────────────────────────┘
```

The shell has three jobs only:

1. Tell the user which project is open.
2. Provide stable navigation.
3. Expose universal actions such as search, undo/redo, save/session state and AI command access.

AI command access is a universal entry point; it does not add a separate product navigation hierarchy.

The shell must not become a dashboard containing production metrics from every module.

## 1.2 Left navigation
The left navigation contains:

- Home
- Idea Vault
- Story
- Screenplay
- Breakdown
- Production
- Call Sheets
- Files

Production contains sub-workspaces for Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Shooting Schedule, Daily Production View, Sides, Reports and Lightweight Budget.

The navigation distinguishes the current workspace using one clear selected state. It does not create permanent top-level navigation items for each small catalog type.

## 1.3 Project selector
The project selector appears in the top bar. Clicking it opens a compact list of recent projects, pinned projects and an All/Archived area.

Switching projects follows the FSD unsaved-work rule. If work is saving or unsaved, the user sees a small confirmation surface before leaving.

The selector makes the current project obvious without using an oversized title block.

## 1.4 Global status strip
The bottom status strip is compact and quiet. It may show:

- Saved
- Saving
- Save error
- Offline
- Local collaboration session
- External AI connection state
- Pending exchange/import operation

The strip must not block creative work for transient status changes.

## 1.5 Universal command behavior
The same commands use consistent labels and placement throughout the application:

- New
- Open
- Duplicate
- Rename
- Delete
- Undo
- Redo
- Import
- Export
- Search

Context menus add object-specific actions but should not invent a different interaction model for each module.

# 2. Shared Interaction Patterns

## 2.1 Cards
Cards are the central visual language for ideas, beats, scenes, shots and several production objects. A card shows only the information needed for its current purpose. Cards can be compact, selectable, draggable, expandable and context-clickable. Differences between card types should come mainly from title treatment, spacing and small visual cues rather than unrelated UI languages.

## 2.2 Expanded detail
When a compact object needs more information, use an expanded drawer or focused detail panel rather than replacing the entire workspace. This is the preferred pattern for Scene Cards, Idea Vault items, catalog items and locations.

## 2.3 Modal dialogs
Use a modal for deliberate decisions, creation, import/export setup, conflict confirmation or destructive actions. Routine editing should happen directly on the page or in a drawer. Avoid stacked modals.

## 2.4 Context menus
Right-click exposes only actions meaningful to the selected object. Common object actions appear first; destructive actions remain at the bottom and are visually separated.

## 2.5 Inline editing
Titles, short descriptions, sequence names, notes and simple fields are editable inline where practical. Enter confirms; Escape cancels the current edit.

## 2.6 Selection
Click selects one object. Shift/Cmd/Ctrl selection extends or toggles selection where multi-select is supported. Selected objects use a clear focus treatment.

## 2.7 Drag-and-drop
Dragging shows an insertion line or destination highlight. Invalid release returns the object to its original location without mutation. Valid release commits immediately and can be undone.

## 2.8 Undo/redo
Meaningful editing and arrangement actions are undoable. The undo label should use human language such as “Move Scene Card” or “Delete Shot.”

## 2.9 Recoverable deletion
Important creative/production objects follow the FSD recoverable-deletion rules. Deletion messaging explains what downstream data remains unchanged.

## 2.10 Empty states
An empty page explains what the workspace is for and presents one or two useful actions. It never shows fake zeros or analytics.

## 2.11 Error states
Errors explain what happened and what to do next. Prefer inline banners and focused dialogs over blocking generic errors.

## 2.12 Save feedback
The shell communicates Saved, Saving or Save Error. Save errors do not discard the current edit.

## 2.13 Source banner
Workspaces derived from a screenplay show a compact source-version banner such as `Production Source: Shooting Draft 6`.

## 2.14 Stale state
A stale document remains readable. The UI identifies the changed source and provides Review/Refresh rather than forcing an automatic mutation.

## 2.15 Export
Export is visible from the relevant workspace and states that it creates a snapshot and does not mutate the project.

## 2.16 Keyboard
Keyboard shortcuts accelerate frequent actions. Core workflows also remain usable with pointer interaction.

## 2.17 Tooltips
Tooltips explain unfamiliar icons. Important functions should still have text labels.

## 2.18 Confirmation language
Confirmation wording states the consequence. Example: `Delete this Scene Card? Its linked screenplay scene will remain.`

# 3. Visual Design System

## 3.1 Visual hierarchy
Use four visual levels:

1. Application shell — persistent and quiet.
2. Workspace header — page title and primary action.
3. Primary work area — editor, cards, strips, tables or panels.
4. Secondary information — notes, counts, source labels and optional metadata.

The object the user came to manipulate should normally dominate the workspace.

## 3.2 Density by workflow
Idea Vault can be visually rich and moderately dense.

Story Board should be dense enough to show many cards at once.

Screenplay should be visually quiet and reading-focused.

Breakdown should use a balanced two/three-pane layout.

Schedule should tolerate high information density with consistent strips and rows.

Call Sheet should prioritize document readability over application chrome.

## 3.3 Status language
Use a small, consistent vocabulary:

- Ready
- Needs attention
- Draft
- Locked
- Stale
- Unscheduled
- Complete
- Archived

Do not invent micro-statuses merely to make a dashboard look comprehensive.

## 3.4 Color semantics
Board colors and personal organization colors are optional. System colors such as production revision colors must remain distinguishable from personal color choices and be accompanied by textual meaning.

## 3.5 Typography
The screenplay editor follows professional screenplay formatting. Other screens use straightforward interface typography. Small cards prioritize legibility and compactness; long creative text wraps in detail views rather than forcing the filmmaker to edit for layout.

## 3.6 Icons and labels
Use icons to support frequent actions, but pair important actions with text. Icon-only buttons are appropriate for compact secondary controls, not the main workflow.

## 3.7 Dialog sizing
Use small dialogs for simple confirmation, medium dialogs for create/import/export, and large dialogs for comparison, exchange review and conflict resolution. Avoid layered dialogs.

# 4. Application Launch and Home
Home is the application-level starting point before a project is open. It helps a filmmaker continue work immediately rather than forcing configuration.

## Primary question
> **Which project should I continue or open?**

## Entry points
- Launching OpenFrame
- Closing a project
- Returning from Project Home

## Recommended layout
Recent Projects occupies the main body. Pinned Projects are visually separated. Archived projects have their own entry rather than appearing in the recent list.

Each project card shows title, type, status and last modified. `New Project` is the dominant action.

No production metadata appears on application Home.

## Visible controls
- New Project
- Open Project
- Search Projects
- Pin/Unpin
- Archive
- Open Archived
- Open existing project

## User actions and behavior
Clicking New Project opens the minimal project dialog. Opening a project enters Project Home. Pinning changes presentation order only. Archiving removes the project from normal recent views but leaves it accessible.

Search is project-level navigation, not screenplay text search.

If there are no projects, the primary action is Create Project.

## Visible states
- No projects
- Recent projects
- Search results
- Archived projects
- Project unavailable

## Empty state
Empty state: `Create your first project.` with a secondary `Open Existing Project` action.

## Errors and recovery
If the known project location is unavailable, explain that the project could not be opened and allow the user to locate/open another copy. Never silently generate a replacement project.

## Cross-module connections
Project Home is the project-specific destination. Global Idea Vault remains available outside a project.

## Keyboard / desktop shortcuts
- Ctrl/Cmd+N — New Project
- Ctrl/Cmd+O — Open Project

## Not on this screen
- Production metadata
- Enterprise dashboards
- Notification center

# 5. Project Home
Project Home is the front door to a film and must remain useful without becoming a metrics dashboard.

## Primary question
> **What do I want to continue doing in this film?**

## Entry points
- Opening a project
- Selecting the project from the shell

## Recommended layout
Top: project title, type, status, last opened.

Main: Continue section with the last meaningful workspaces.

Quick Access: Idea Vault, Story Board, Screenplay, Breakdown, Production, Call Sheets.

Lower: Recent and Project Files.

Production status indicators appear only after production content exists.

## Visible controls
- Continue
- Idea Vault
- Story Board
- Screenplay
- Breakdown
- Production
- Call Sheets
- Files

## User actions and behavior
Continue remembers the last meaningful location. New projects show three high-value next actions: Open Idea Vault, Build Story, Write/Import Screenplay.

Once production begins, lightweight status can show breakdown progress, next shooting day, outstanding location/cast needs and latest call sheet.

## Visible states
- Brand-new project
- Development
- Writing
- Production
- Archived

## Empty state
A new project shows no fake metrics. It explains the three main starting paths.

## Errors and recovery
If a recent document is unavailable, the page remains usable and directs the user to the correct workspace. Save errors follow global save feedback behavior.

## Cross-module connections
All major workspaces are linked but their data is not duplicated here.

## Keyboard / desktop shortcuts
- Home shortcut
- Open Project

# 6. New Project Dialog
New Project is deliberately minimal.

## Primary question
> **What is this project called and what type is it?**

## Entry points
- Home > New Project
- + command > New Project

## Recommended layout
Compact centered dialog.

Required: Title, Type.

Optional: Language, Genre, Creator.

Optional fields are secondary and do not block creation.

## Visible controls
- Title
- Type
- Language
- Genre
- Creator
- Create
- Cancel

## User actions and behavior
Create becomes enabled when required values are valid. After creation, the user enters Project Home immediately. There is no setup wizard.

Project type options are Feature Film, Short Film and Episodic/Series.

## Visible states
- Blank
- Typing
- Invalid
- Creating
- Created
- Creation error

## Empty state
Under the fields, a small sentence says that more information can be added later.

## Errors and recovery
If creation fails, retain entered values and offer Retry/Cancel. No partially created empty project should appear as if complete.

## Cross-module connections
Project type determines the initial project context but does not remove core navigation.

# 7. Global Idea Vault
The Global Idea Vault is the permanent creative dump outside a specific film.

## Primary question
> **What do I have that might matter someday?**

## Entry points
- Global navigation > Idea Vault
- Home > Idea Vault without project open

## Recommended layout
Header: `Idea Vault` + Global/Project scope selector.

Toolbar: Add, Search, View switcher, organization controls.

Body: Visual Grid, Card View, List View or Folder View.

Optional side region: current folder/collection.

## Visible controls
- Add
- Drag in
- Search
- Switch view
- Folder
- Collection
- Pin
- Copy to Project
- Delete

## User actions and behavior
The user can add text, images, URLs, PDFs, documents, audio/voice notes, video, handwritten/sketch images, quotes, screenshots and supported files.

No classification is required. Untitled items are allowed.

Copying to a project creates an independent project copy.

## Visible states
- Empty
- Mixed items
- Selected item
- Search
- Folder open
- External file unavailable

## Empty state
`Drop any film idea, reference or file here.` Primary action: Add.

## Errors and recovery
Missing external references remain visible with an unavailable indicator. Failed file additions report only the failed items and keep successful additions.

## Cross-module connections
Global Vault can copy material to Project Vault and can send material into Story, but it never becomes a live sync layer for screenplay content.

## Keyboard / desktop shortcuts
- Ctrl/Cmd+F — Search
- Delete — selected item
- Enter — open item

# 8. Project Idea Vault
The Project Idea Vault is the film-specific creative dump.

## Primary question
> **What have I collected for this film?**

## Entry points
- Project Home > Idea Vault
- Project shell > Idea Vault

## Recommended layout
Same visual language as Global Idea Vault, but the scope is clearly marked `Current Project`.

Pinned items can appear at the top. Collections are optional.

## Visible controls
- Add
- Search
- Grid/Card/List/Folder
- New Collection
- Pin
- Copy to Global
- Send to Story
- Delete

## User actions and behavior
The user can store any reference or creative material relevant to the film without classification. Copy from Global to Project creates an independent item.

The user can move/copy a useful item into Story as Beat, Scene Card, Sequence idea, Character note or Story note.

## Visible states
- Empty
- Mixed
- Pinned
- Selected
- Search
- Source unavailable

## Empty state
`Start collecting anything about this film.`

## Errors and recovery
The same recovery rules as Global Vault apply.

## Cross-module connections
This workspace remains independent from Story Board and Screenplay.

# 9. Idea Vault Item Detail / Preview
Preview lets the filmmaker inspect or edit one item without forcing structure.

## Primary question
> **What is this item and what do I want to do with it?**

## Entry points
- Double-click item
- Context menu > Open

## Recommended layout
Large detail drawer.

Media preview occupies the left/center. A compact metadata column contains title, body/caption, folder, collections, tags, pin and source where applicable.

## Visible controls
- Edit
- Rename
- Move
- Add to Collection
- Pin
- Copy to Project
- Send to Story
- Open externally
- Delete

## User actions and behavior
Text opens directly into editing. Images/PDFs/documents/audio/video show a suitable local preview when possible and an external-open action when necessary.

Send to Story asks for destination object type. The created Story object is a copy; the original remains.

## Visible states
- Preview
- Editing
- Saving
- External unavailable
- Deleted/recoverable

## Empty state
An untitled item remains valid. The interface does not force naming before closing.

## Errors and recovery
If the primary file is missing or inaccessible, preserve the item record and show a clear unavailable/relink path where applicable.

## Cross-module connections
This detail view provides context without establishing live relationships to Story or Screenplay.

# 10. Idea Vault Quick Capture
Quick Capture is optimized for the moment a filmmaker has a sudden thought.

## Primary question
> **How do I capture this idea before I forget it?**

## Entry points
- Global Quick Capture action
- Project quick action

## Recommended layout
A tiny text capture window or palette with one multiline input and Save/Cancel.

Project context is shown when a project is active.

## Visible controls
- Type
- Save
- Cancel
- Choose project only when needed

## User actions and behavior
Saving creates a text note in the current Project Vault or selected Global Vault context. No title or classification is required.

The capture closes immediately after save.

## Visible states
- Empty
- Typing
- Saved
- Save error

## Empty state
The field is blank and focused automatically.

## Errors and recovery
Save errors retain entered text until the user retries or cancels.

## Cross-module connections
Quick Capture creates a normal Idea Vault item; it does not create a Scene Card or screenplay content automatically.

## Keyboard / desktop shortcuts
- Enter/Ctrl+Enter — save if chosen
- Escape — cancel

# 11. Story Board — Board View
The Story Board is the primary visual outlining surface.

## Primary question
> **How does my story fit together?**

## Entry points
- Project Home > Story
- Shell > Story

## Recommended layout
Header: Story Board, Board/Outline switcher, Add menu, search/filter, optional zoom, Undo/Redo.

Main canvas: Acts separated visually; Sequences grouped inside Acts; Scene Cards arranged within Sequences; Beats may appear where valid.

Parking Lot is a collapsible side area.

## Visible controls
- Add Act
- Add Sequence
- Add Beat
- Add Scene
- Board/Outline
- Zoom
- Search/Filter
- Multi-select
- Parking Lot
- Build Screenplay

## User actions and behavior
Dragging an object shows insertion/destination feedback and commits immediately on valid drop. Invalid release returns it to original position.

Scene Cards remain compact rectangles. The board does not expose production metadata.

The board supports multi-select, duplicate, move, Parking Lot and experimental ordering.

## Visible states
- Empty
- Populated
- Dragging
- Filtering
- Card expanded
- Saving
- Save error
- Build preview

## Empty state
Use the PRD copy:

> `Scene Cards are short reminders of what happens in each scene. Drag them until the story works.`

## Errors and recovery
Drag failure, save failure and invalid targets never silently change the data.

## Cross-module connections
Story ordering can be used for Build Screenplay, but later Story Board edits do not automatically rewrite screenplay content.

## Keyboard / desktop shortcuts
- Ctrl/Cmd+Z — undo
- Ctrl/Cmd+Shift+Z — redo
- Arrow keys — navigate selected items where natural

# 12. Story Board — Outline View
Outline View exposes the same story structure as a compact hierarchy.

## Primary question
> **Where does each Act, Sequence, Beat and Scene Card sit?**

## Entry points
- Story Board > Outline

## Recommended layout
Indented rows:

Act
  Sequence
    Beat
    Scene Card

Each row has a drag handle and expand/collapse affordance where applicable.

## Visible controls
- Add
- Rename
- Drag
- Move
- Duplicate
- Park
- Delete
- Build Screenplay

## User actions and behavior
Editing in Outline View changes the same story objects as Board View. It is optimized for hierarchy and order, not additional metadata entry.

## Visible states
- Empty
- Expanded
- Collapsed
- Dragging
- Filtered

## Empty state
A concise hierarchy starter is shown when empty.

## Errors and recovery
Invalid hierarchical drops are rejected with an explanatory inline message.

## Cross-module connections
No screenplay text or production fields appear here.

# 13. Act
Acts provide high-level story grouping.

## Primary question
> **What major section of the story am I looking at?**

## Entry points
- Story Board > Add Act
- Context menu

## Recommended layout
Act title is visually prominent and its children appear beneath it. Collapse/expand is prominent.

## Visible controls
- Add Act
- Rename
- Collapse
- Expand
- Move
- Delete
- Add Sequence
- Add Scene

## User actions and behavior
Create requires only title. Dragging an Act moves all child sequences/scenes. Delete of a non-empty Act offers child reassignment or explicit deletion.

Act rename does not edit screenplay text.

## Visible states
- Empty
- Populated
- Collapsed
- Delete confirmation

## Empty state
`Add a sequence or scene.`

## Errors and recovery
Deletion warnings explain exactly what is retained.

## Cross-module connections
Acts are Story Board structures, not screenplay text elements.

# 14. Sequence
Sequences are simple named containers for groups of scenes.

## Primary question
> **What group of scenes belongs together?**

## Entry points
- Story Board > Add Sequence

## Recommended layout
Sequence header shows name, child count, collapse and context menu. One text field dominates when creating it.

## Visible controls
- Add Sequence
- Rename
- Move
- Collapse
- Expand
- Delete

## User actions and behavior
Create with a single text field. Scene Cards are normal children; Beats may temporarily exist inside a sequence.

Dragging between Acts changes parent while preserving identities.

## Visible states
- Empty
- Populated
- Dragging

## Empty state
`Drop scenes here.`

## Errors and recovery
Invalid moves do not mutate. Deleting a populated sequence uses the safe child-reassignment/delete flow.

## Cross-module connections
Sequence grouping remains Story Board reference structure; it is not automatically inserted into screenplay text.

# 15. Beat Card
Beat Cards represent small events/ideas that may later become scenes.

## Primary question
> **What happens next in the story?**

## Entry points
- Story Board > Add Beat
- Idea Vault > Send to Story as Beat

## Recommended layout
Compact draggable rectangle with one primary text field. Optional note/color/reference appear only in expanded state.

## Visible controls
- Add
- Edit
- Duplicate
- Move
- Convert to Scene
- Park
- Delete

## User actions and behavior
The user types directly. Convert to Scene creates a new Scene Card from the beat text and retains the original beat in the converted/reference state according to FSD behavior.

## Visible states
- New
- Editing
- Dragging
- Converted
- Parked

## Empty state
A blank in-progress card can exist while being typed, but useful saved beats need text unless intentionally parked.

## Errors and recovery
Recoverable deletion/invalid drag follow shared patterns.

## Cross-module connections
Beat content does not become screenplay content unless converted and intentionally included.

# 16. Scene Card
Scene Cards are the compact scene reminders used for visual story shaping.

## Primary question
> **What happens in this scene?**

## Entry points
- Story Board > Add Scene
- Convert Beat
- Send Idea to Story as Scene Card

## Recommended layout
Collapsed card:

```text
┌───────────────────────────────┐
│ Arjun discovers the missing   │
│ file...                       │
└───────────────────────────────┘
```

Only short description dominates. A tiny heading indicator/comment count may appear.

No scene number or production metadata on the card.

## Visible controls
- Open
- Edit
- Expand
- Duplicate
- Move
- Park
- Convert to Screenplay Scene
- Comment
- Export
- Delete

## User actions and behavior
Single click selects; double-click expands. Drag/drop reorders/reparents.

Duplicate creates a new identity and no screenplay relationship.

Delete warns if linked to a screenplay scene; the screenplay scene remains.

Multi-select is supported where appropriate.

## Visible states
- Compact
- Selected
- Dragging
- Expanded
- Parked
- Commented

## Empty state
Use short instructional text only on the empty board, not on every card.

## Errors and recovery
Deletion warning must state that screenplay is not automatically deleted.

## Cross-module connections
Scene Card changes after screenplay creation do not rewrite screenplay text.

## Keyboard / desktop shortcuts
- Enter — open details
- Escape — close detail

# 17. Scene Card Detail
Expanded Scene Card gives the user enough room to clarify a scene without turning it into a database form.

## Primary question
> **What do I currently mean by this scene?**

## Entry points
- Scene Card > Open

## Recommended layout
Drawer:

Heading (optional)
Short Description
Notes
Attachments
Comments

A small location label shows Act/Sequence context.

## Visible controls
- Edit heading
- Edit description
- Edit notes
- Attach
- Comment
- Duplicate
- Move
- Park
- Delete
- Convert

## User actions and behavior
Heading remains optional in Story Board. Build Screenplay is responsible for validating heading before conversion.

## Visible states
- Editing
- Saved
- Saving
- Comment thread

## Empty state
Description is the primary field and should receive focus on a newly created card.

## Errors and recovery
No production or character fields are introduced here.

## Cross-module connections
The card remains the same Story Board identity throughout.

# 23. Screenplay — Standard Mode
Professional writing surface with minimal distractions.

## Primary question
> **What is the movie on the page?**

## Entry points
- Project Home > Write
- Open screenplay
- Open draft

## Recommended layout
Three zones:

Left: compact scene navigator.

Center: screenplay page.

Right: optional tool panel.

Top: draft selector, Review, Compare, Export, Lock/Revision where applicable.

The script occupies most of the central area.

## Visible controls
- New Scene
- Element selector
- Scene navigator
- Search
- Comments
- Notes
- Drafts
- Compare
- Export
- Lock/Revision

## User actions and behavior
Normal text editing uses screenplay formatting. Scene numbers derive from scene order.

The scene navigator jumps to scenes without opening another document. Search uses standard text search in the current screenplay.

## Visible states
- Loading
- Editing
- Searching
- Comparing
- Review overlay
- Locked
- Revision
- Saving
- Save error

## Empty state
New screenplay: blank page + `Start writing or use Build Screenplay.`

## Errors and recovery
Saving problems preserve the local editing state. Search and export failures are localized.

## Cross-module connections
Screenplay is authoritative written content; Story Board and Idea Vault are separate reference surfaces.

## Keyboard / desktop shortcuts
- Ctrl/Cmd+F
- Ctrl/Cmd+S
- Ctrl/Cmd+Z
- Ctrl/Cmd+Shift+Z

# 24. Screenplay — Focus Mode
Fullscreen writing mode removes application chrome and optional panels.

## Primary question
> **Can I write without distractions?**

## Entry points
- Screenplay > Focus Mode
- Fullscreen Writing

## Recommended layout
The screenplay page fills nearly the entire window. A very small control returns to normal mode.

## Visible controls
- Exit Focus
- Minimal draft/status indicator

## User actions and behavior
Entering and exiting does not mutate content or layout data. The previous normal layout is restored when leaving.

## Visible states
- Focus
- Typing
- Saving
- Save error

## Empty state
No additional onboarding in focus mode.

## Errors and recovery
Save errors use a discreet status unless immediate recovery is needed.

## Cross-module connections
Focus Mode is presentation only; screenplay behavior remains identical.

# 25. Writing Room
Writing Room is a layout combining screenplay with one or two optional context panels.

## Primary question
> **What reference do I need while writing this scene?**

## Entry points
- Screenplay > Writing Room

## Recommended layout
Left: scene navigator.

Center: screenplay.

Right: optional panel area with choices for Story Board reference, Scene Notes, Characters, Comments or Draft Information.

## Visible controls
- Open panel
- Switch panel
- Resize
- Close panel
- Focus Mode

## User actions and behavior
Only one or two panels should normally be visible. Panels can be resized and closed. The Story Board panel is reference-only and does not create live synchronization.

## Visible states
- No panel
- One panel
- Two panels
- Focus

## Empty state
If no panel is selected, center the screenplay and keep empty side space quiet.

## Errors and recovery
If a panel fails, leave the screenplay fully usable.

## Cross-module connections
Idea Vault remains accessible through normal navigation, not as a permanent Writing Room sidebar.

# 26. Screenplay Navigation and Search
Scene navigation and ordinary text search keep the editor fast without semantic analysis.

## Primary question
> **What scene or text do I want to find?**

## Entry points
- Scene navigator
- Ctrl/Cmd+F

## Recommended layout
Left scene navigator lists generated scene number and heading. Search opens a compact find bar over the screenplay.

## Visible controls
- Search
- Next
- Previous
- Replace
- Case sensitive
- Whole word
- Scene navigation

## User actions and behavior
Search operates on the current screenplay. Clicking a scene navigates to it. Replace respects selected options and remains undoable.

## Visible states
- No match
- Match found
- Replacing
- Search closed

## Empty state
No match shows `No matches found` without leaving the scene.

## Errors and recovery
Replace failures must not partially alter text.

## Cross-module connections
Global project search remains separate from current screenplay search.

## Keyboard / desktop shortcuts
- Ctrl/Cmd+F
- Enter — next match
- Shift+Enter — previous match
- Escape — close find bar

# 27. Named Drafts and Automatic History
Versioning separates meaningful drafts from quiet recovery history.

## Primary question
> **Which version am I working on, and what can I restore if needed?**

## Entry points
- Screenplay > Drafts

## Recommended layout
Draft drawer lists named drafts with date, current/locked status and optional note. Automatic history is separated under `Recovery History`.

## Visible controls
- New Draft
- Open
- Compare
- Rename
- Mark current
- Restore recovery

## User actions and behavior
New named draft is created from the current/selected source and leaves the source unchanged. Automatic history points are recovery-oriented and should not clutter normal draft selection.

## Visible states
- One draft
- Multiple
- Current
- Locked
- Recovery available

## Empty state
Initial screenplay has one clear current draft. Explain recovery history only when relevant.

## Errors and recovery
If a draft is unavailable, preserve the currently open draft and show recovery/retry.

## Cross-module connections
Review rounds and locks reference named drafts; Story Board remains separate.

# 28. Draft Comparison
Side-by-side or change-focused comparison.

## Primary question
> **What exactly changed?**

## Entry points
- Drafts > Compare

## Recommended layout
Change navigator on one side, Draft A and Draft B in split panes.

Top summary: scenes added, removed, changed; changed lines.

## Visible controls
- Choose A/B
- Next change
- Previous
- Scene-only
- Text-only
- Close

## User actions and behavior
Selecting a change jumps both documents. Added and removed scenes are called out. Text comparison is human-readable.

## Visible states
- No differences
- Changes
- Added scene
- Removed scene
- Modified scene

## Empty state
If no differences, say `No differences found.`

## Errors and recovery
Missing drafts produce a selection error and do not alter them.

## Cross-module connections
Comparison is read-only.

# 29. Review Round
Structured feedback on a selected draft.

## Primary question
> **What feedback is still open?**

## Entry points
- Screenplay > Review
- Draft History > Create Review

## Recommended layout
Header: review name, source draft and reviewers.

Body: comments grouped by scene/status.

Side panel: selected comment thread.

## Visible controls
- New Review
- Select draft
- Add reviewer
- Open
- In Discussion
- Resolve
- Reopen
- Reply

## User actions and behavior
Review requires source draft and name. Comments attach to exact text or scene objects. Resolved comments remain in history.

## Visible states
- No comments
- Open
- Resolved
- Reviewer present
- Closed

## Empty state
New review explains how to add the first note.

## Errors and recovery
Target loss/mapping uses the FSD comment persistence rules.

## Cross-module connections
Review rounds link to draft identity and exchange packages.

# 30. Comments and Private Notes
Shared comment interaction and strictly private note behavior.

## Primary question
> **What do I want to tell someone about this exact work?**

## Entry points
- Text selection
- Scene/object comment
- Notes panel

## Recommended layout
Text comments appear as anchored bubbles and in the comment panel. Object comments appear in a thread. Private notes use a distinct private indicator.

## Visible controls
- Comment
- Reply
- Resolve
- Reopen
- Private Note
- Delete

## User actions and behavior
Comments support screenplay text, scenes, beats, sequences, acts, storyboard panels, shots, locations and breakdown items where defined.

Private notes never enter normal exports/exchanges.

## Visible states
- Draft comment
- Posted
- Resolved
- Private
- Target deleted

## Empty state
Empty thread: `No comments yet.`

## Errors and recovery
If an imported comment cannot be mapped safely, keep it in Review Queue/Unmapped Review Note.

## Cross-module connections
Comments do not mutate source text automatically.

# 31. Script Lock and Production Revisions
Production safety state.

## Primary question
> **Am I editing the approved shooting draft or a new production revision?**

## Entry points
- Screenplay > Lock
- Edit locked draft

## Recommended layout
Status area identifies Draft / Locked Shooting Draft / Revision.

Lock dialog summarizes draft and open review notes. Revision panel shows label, color and changed scenes.

## Visible controls
- Lock
- Start Revision
- Set Revision Label/Color
- Compare
- View History

## User actions and behavior
Lock keeps the source readable and protected from silent editing. Editing prompts Start Revision. Revision colors apply only to production revisions.

## Visible states
- Unlocked
- Lock dialog
- Locked
- Revision mode
- Saving

## Empty state
Lock is described as a safety state, not a destructive action.

## Errors and recovery
Failure leaves the source unchanged.

## Cross-module connections
Production can select the locked/revised draft as source.

# 32. Screenplay Import
Safe existing-script import.

## Primary question
> **What existing script do I want to bring into OpenFrame?**

## Entry points
- Project Home > Import
- Screenplay > Import

## Recommended layout
Three-step layout: Select source → Preview interpretation → Import.

Preview lists detected scenes, characters, page count, title and warnings.

## Visible controls
- Choose file
- Paste text
- Preview
- New screenplay
- New draft
- Import
- Cancel

## User actions and behavior
Supported sources: PDF, FDX, Fountain, TXT, DOCX and pasted screenplay text. Existing drafts are not overwritten by default.

Uncertain parsing is presented as a warning/review state.

## Visible states
- Selecting
- Parsing
- Preview
- Warnings
- Ready
- Importing
- Complete
- Failed

## Empty state
The preview helps the user understand how the source was interpreted before committing.

## Errors and recovery
Import failure leaves existing project content untouched.

## Cross-module connections
Imported screenplay becomes screenplay/draft content and does not automatically create Story Board or production data.

# 33. Screenplay Export and Print
Professional document output.

## Primary question
> **How do I export this screenplay as a clean document?**

## Entry points
- Screenplay > Export
- File > Print

## Recommended layout
Export configuration includes source draft, format, scope and optional output settings. Print Preview shows the outgoing document.

## Visible controls
- PDF
- FDX
- Fountain
- DOCX
- Scope
- Title page
- Revision info
- Include notes
- Preview
- Export
- Print

## User actions and behavior
Export is a snapshot and does not mutate the project. Private notes are excluded by default.

## Visible states
- Ready
- Preview
- Exporting
- Complete
- Failure

## Empty state
The dialog clearly identifies the source draft and inclusion choices.

## Errors and recovery
Export failures leave the project unchanged.

## Cross-module connections
Normal exports are distinct from exchange packages.

# 34. Document Preview and Professional Identity
Review the actual outgoing document before sending it.

## Primary question
> **Does this document look correct before I send it?**

## Entry points
- Export/Print Preview

## Recommended layout
Document pages dominate. A compact configuration area controls title page, page numbering, date and revision information where relevant.

## Visible controls
- Page navigation
- Zoom
- Title page
- Page numbers
- Revision info
- Include notes
- Export
- Print

## User actions and behavior
Changing preview settings affects only the outgoing snapshot. It never changes screenplay content.

## Visible states
- Rendering
- Preview ready
- Long document
- Revision preview

## Empty state
Preview always has a clear Back/Close action.

## Errors and recovery
Render failure does not modify the source document.

## Cross-module connections
Project identity supplies document fields but the preview is presentation-only.

# 35. Additional File Interchange
P2 interchange formats should remain out of the main workflow until needed.

## Primary question
> **Do I need an additional file format?**

## Entry points
- Export > Additional Formats

## Recommended layout
Collapsed advanced area below core PDF/FDX/Fountain/DOCX choices.

## Visible controls
- Expand
- Select format
- Export

## User actions and behavior
Advanced formats use the same export preview and snapshot rules. They must not replace the core output choices.

## Visible states
- Collapsed
- Expanded
- Unavailable
- Supported

## Empty state
No additional-format area should dominate the normal screen.

## Errors and recovery
Unsupported formats remain visibly unsupported and never block core exports.

## Cross-module connections
This maps to PRD-SCRIPT-007 P2.

# 36. Episodic / Series
Episodic projects organize seasons and episodes without a giant series database.

## Primary question
> **Which episode am I working on?**

## Entry points
- Episodic project
- Project Home > Series

## Recommended layout
Series header, season selector, episode list. Each episode row shows title, optional one-line summary and status.

Selecting an episode enters the same Story/Screenplay/Production workspaces in episode context.

## Visible controls
- Add Season
- Add Episode
- Rename
- Open Episode
- Summary
- Status

## User actions and behavior
Each episode may have its own Idea Vault subset, Story Board, screenplay drafts, breakdown and production plan. A small series-level character/location reference can be available without forcing cross-episode continuity management.

## Visible states
- No episodes
- Season with episodes
- Episode selected

## Empty state
`Create your first episode.`

## Errors and recovery
Deletion or invalid episode operation must preserve other episodes.

## Cross-module connections
P2 deeper continuity is not shown as a required dashboard.

# 37. Deeper Episodic Continuity
P2 cross-episode continuity can be exposed later as a simple reference view.

## Primary question
> **Where does this recurring character or location continue across episodes?**

## Entry points
- Series > optional continuity view

## Recommended layout
Simple filtered list of recurring characters/locations/items with episode references. No giant matrix.

## Visible controls
- Filter
- Open episode
- Open character/location
- Add note

## User actions and behavior
Continuity information is reference-only. It does not block writing or automatically change episodes.

## Visible states
- No recurring items
- Recurring item
- Filtered

## Empty state
If no continuity data exists, keep this view hidden from normal navigation.

## Errors and recovery
Continuity warnings are advisory only.

## Cross-module connections
This is P2 and must not expand into series ERP.

# 38. Scene Hub
Scene Hub provides a compact cross-module view around one scene.

## Primary question
> **What exists around this scene?**

## Entry points
- Screenplay scene > Scene Hub
- Search result

## Recommended layout
Header: Scene number + heading.

Link/tabs:
Screenplay
Breakdown
Shots
Storyboard
Schedule
Comments
Story Board

Each can show a count/status where useful.

## Visible controls
- Open Screenplay
- Open Breakdown
- Open Shots
- Open Storyboard
- Open Schedule
- Open Comments
- Open Story Board

## User actions and behavior
Selecting a link opens the dedicated workspace focused on the same scene. The hub can show simple statuses like `Scheduled: Shoot Day 7` without displaying all production fields.

## Visible states
- Only screenplay
- Some production data
- Stale source
- Nothing created

## Empty state
`Nothing created yet` with the next useful creation action.

## Errors and recovery
If a linked workspace cannot open, keep the hub intact and identify the unavailable destination.

## Cross-module connections
Scene Hub is navigation and context, not a master edit screen.

# 39. Breakdown
Breakdown is the bridge from screenplay to practical production requirements.

## Primary question
> **What does this scene require to shoot?**

## Entry points
- Breakdown navigation
- Scene Hub > Breakdown
- Production source selected

## Recommended layout
Left: scene list.

Center: screenplay reading pane.

Right: confirmed breakdown categories and suggested items.

Top: compact Production Source banner.

## Visible controls
- Select source
- Choose scene
- Highlight text
- Tag
- Add manual
- Suggest
- Accept/Edit/Reject
- Catalog
- Complete
- Filter

## User actions and behavior
Manual tagging works from highlighted text or manual entry. Suggestions remain unconfirmed until user action. Default categories stay limited to Cast, Extras/Background, Location/Set, Props, Wardrobe, Vehicles, Hair/Makeup, Special Effects, VFX, Sound and Animals.

## Visible states
- No Source
- Loading
- Reviewing Scene
- Suggesting
- Suggestions Ready
- Complete
- Needs Review
- Save Error

## Empty state
`Choose a screenplay source to begin the breakdown.`

## Errors and recovery
A script revision never silently deletes breakdown data. Affected scenes are flagged for review.

## Cross-module connections
Accepted elements populate the catalog; screenplay text remains unchanged.

# 40. Breakdown Suggestion Review
Suggested elements are reviewed like a checklist, never silently committed.

## Primary question
> **Which suggestions do I want to accept?**

## Entry points
- Breakdown > Suggest

## Recommended layout
Suggestion list grouped by category. Each row has a selection control, suggested name, optional matched text and decision controls.

## Visible controls
- Accept
- Reject
- Edit
- Accept selected
- Accept all safe
- Dismiss

## User actions and behavior
Editing a suggestion changes its proposed production data before acceptance.

Existing Catalog matches are suggested where obvious. Ambiguous matches open a chooser. No-match suggestions can become new catalog items.

## Visible states
- Loading
- Ready
- Edited
- Accepted
- Rejected
- Ambiguous

## Empty state
No suggestions is a normal state with a reminder that manual tagging remains available.

## Errors and recovery
If suggestion service fails, manual breakdown remains usable.

## Cross-module connections
AI/suggestion provenance is visible where applicable. User decision remains authoritative.

# 41. Production Catalog
Catalog is the reusable directory of things and people needed by production.

## Primary question
> **What things does this film need?**

## Entry points
- Production > Catalog
- Breakdown > Catalog

## Recommended layout
Searchable list/table with optional category filter. Detail drawer shows name, category, status, image, notes and scene usage.

## Visible controls
- Search
- Filter
- Add Item
- Open
- Rename
- Archive
- Open Scenes

## User actions and behavior
Confirmed breakdown elements can create/use catalog items. Opening an item lists every associated scene. Removing one scene association leaves the catalog item intact.

Renaming updates active UI references while historical document snapshots stay unchanged.

## Visible states
- Empty
- Populated
- Filtered
- Archived

## Empty state
`Confirmed breakdown items will appear here, or add one manually.`

## Errors and recovery
If an item is archived while still referenced, explain the effect before completing the action.

## Cross-module connections
Catalog items are shared identity objects used by multiple breakdown scenes; they are not duplicated per scene.

# 42. Locations
Location workspace is a practical scouting/planning notebook.

## Primary question
> **Which places can we use?**

## Entry points
- Production > Locations
- Breakdown location link

## Recommended layout
List/grid with thumbnails, detail drawer.

Detail: Name, Address/Area, Contact, Photos, Notes, Status, Scenes.

## Visible controls
- Add Location
- Edit
- Photos
- Notes
- Status
- Search
- Open Scenes
- Delete

## User actions and behavior
Create requires only name. Status: Idea, Shortlisted, Confirmed, Rejected.

Practical notes include parking, noise, permission, access, power, toilets, nearby facilities and travel notes.

## Visible states
- Empty
- Idea
- Shortlisted
- Confirmed
- Rejected
- Selected

## Empty state
`Add a location you are considering for the film.`

## Errors and recovery
Deletion warns that schedule history remains readable.

## Cross-module connections
Locations link from breakdown to schedule/call sheet and remain separate from screenplay text.

# 43. Cast & Crew
Small-team people directory.

## Primary question
> **Who is involved and what role do they have?**

## Entry points
- Production > Cast & Crew
- Characters > Assign Actor

## Recommended layout
Segmented tabs Cast | Crew.

Cast row: Person, Character, Photo, Contact, Availability note.

Crew row: Person, Role, Department, Contact.

## Visible controls
- Add Person
- Assign Character
- Edit
- Availability
- Search
- Open Scenes
- Archive/Delete

## User actions and behavior
No payroll/HR fields. Cast can be associated with a Character. Crew requires Role/Department.

People can be referenced in schedule and call sheets.

## Visible states
- Empty cast
- Populated cast
- Empty crew
- Selected

## Empty state
Use concise guidance without enterprise language.

## Errors and recovery
Deletion warns about existing references.

## Cross-module connections
Cast connects to breakdown/schedule/call sheet. Crew connects to call sheet and project directory.

# 44. Moodboards
Visual reference boards for director/creative planning.

## Primary question
> **What should this film feel like?**

## Entry points
- Production > Moodboards
- Quick Action > New Moodboard

## Recommended layout
Large canvas with image/note/link tiles. Header: board name, Add, Export.

## Visible controls
- Add image
- Add note
- Add link
- Move
- Resize
- Caption
- Export

## User actions and behavior
Users arrange references freely. Captions optional. Private/internal notes can be excluded from exports.

## Visible states
- Empty
- Populated
- Dragging
- Selected
- Export preview

## Empty state
`Drop images, notes and references here.`

## Errors and recovery
Missing images show an unavailable-reference placeholder.

## Cross-module connections
Moodboards may be referenced by story/production views but never become mandatory production requirements.

# 45. Storyboards
Basic visual storyboard workspace.

## Primary question
> **What will the audience see?**

## Entry points
- Production > Storyboards
- Scene Hub > Storyboard

## Recommended layout
Scene selector/header at top. Main panel is ordered storyboard panels.

Each panel contains visual area and description. Optional shot badge/link.

## Visible controls
- Add Panel
- Import Image
- Draw/Sketch
- Move
- Link Shot
- Export

## User actions and behavior
Panels belong to a selected scene. Reorder by drag. Basic P0 tooling should stay simple. P1 improvements and P2 advanced editing use the same base screen.

## Visible states
- Empty
- Panel
- Image
- Sketch
- Linked shot

## Empty state
`Create panels for this scene.`

## Errors and recovery
Import/drawing issues leave the panel intact and allow another input method.

## Cross-module connections
Storyboard and shot list are independent with optional links.

# 46. Shot List
Practical coverage planning per scene.

## Primary question
> **How will I capture this scene?**

## Entry points
- Production > Shot Lists
- Scene Hub > Shots

## Recommended layout
Scene selector at top, ordered shot cards below. Expanded drawer reveals optional technical fields.

## Visible controls
- Add Shot
- Reorder
- Edit
- Duplicate
- Storyboard link
- Technical details
- Export

## User actions and behavior
Shot requires description only. Number/order derives from position. Optional fields: framing, movement, angle, lens, camera notes, characters, visual reference, storyboard, sound note.

Shot does not require camera jargon to create.

## Visible states
- No shots
- One
- Many
- Selected
- Export preview

## Empty state
`Add only the shots you need. Camera details are optional.`

## Errors and recovery
Incomplete optional technical fields do not block core shot creation.

## Cross-module connections
Shots retain scene association and appear in Scene Hub/Daily context where relevant.

# 47. Shooting Schedule — Stripboard
Primary scheduling workspace using the familiar stripboard model while keeping controls light.

## Primary question
> **When and where will we shoot these scenes?**

## Entry points
- Production > Shooting Schedule
- Scene Hub > Schedule

## Recommended layout
Top: schedule title and source banner.

Main: Unscheduled pool plus Shooting Day columns/rows.

Each strip shows generated scene number, INT/EXT, location, day/night, page count, short synopsis, optional cast indicators and important breakdown indicators.

A small date/calendar control sits above the days.

## Visible controls
- Create Shooting Day
- Drag Scene
- Reorder
- Move Day
- Remove
- Off Day
- Break Marker
- Date
- Suggest Grouping
- Warnings
- Export

## User actions and behavior
Initially, screenplay scenes appear in an Unscheduled pool.

Dragging a scene into a day assigns it to that day. Reordering within a day changes shooting order. Moving to another day reassigns it.

Break markers support Meal, Travel, Company Move and Custom Note.

No automatic optimizer rearranges scenes without explicit user approval.

## Visible states
- No schedule
- Unscheduled Pool
- Planning
- Day selected
- Conflict Warning
- Export Preview
- Saving

## Empty state
`Create a shooting day, then drag scenes into it.`

## Errors and recovery
Invalid drops return a scene to its original position. Schedule save failures preserve the local arrangement.

## Cross-module connections
Schedule scenes reference screenplay source identity and production information. They do not reverse-edit screenplay order or text.

# 48. Shooting Day Detail
Focused planning surface for one shooting day.

## Primary question
> **What exactly are we doing on this day?**

## Entry points
- Click a shooting day
- Schedule > Open Day

## Recommended layout
Header: date, day number.

Center: scene strips in shooting order.

Side panel: locations, cast, estimated duration and day notes derived from assigned scenes.

Footer: total estimated duration and Create Call Sheet.

## Visible controls
- Edit date
- Edit day number
- Open scene
- Add break
- Add note
- Create Call Sheet
- Export

## User actions and behavior
Scene membership is controlled by schedule placement. Derived cast/location information is viewable but should not become a second editing form.

Day notes and break markers are editable here.

## Visible states
- Empty day
- Populated day
- Duration warning
- Source stale

## Empty state
`No scenes scheduled.` plus Add Scene/Off Day options.

## Errors and recovery
If a source scene is stale/removed, show the source-state warning while preserving the day record.

## Cross-module connections
Call Sheet uses the shooting day as source but becomes its own document/snapshot.

# 49. Schedule Assistance
Advisory scheduling suggestions and conflict review.

## Primary question
> **Can I make this day more practical?**

## Entry points
- Schedule > Suggest
- Conflict indicator

## Recommended layout
A compact right-side drawer lists practical observations with explanation.

Examples:
`These 4 scenes use the same location.`
`Actor appears in two locations on this day.`
`Estimated duration exceeds target.`

## Visible controls
- Ask for suggestion
- Review
- Dismiss
- Apply manually
- Keep anyway

## User actions and behavior
Suggestions describe grouping opportunities but never silently move scenes.

Conflict warning types are actor, location and duration overflow. The user can keep a conflict and continue.

## Visible states
- No suggestions
- Suggestion
- Conflict
- Acknowledged

## Empty state
No suggestion is a valid result: `No obvious grouping improvement found.`

## Errors and recovery
If assistance fails, schedule remains unchanged.

## Cross-module connections
Basic schedule assistance is P0. Richer conflict detection is P1. More sophisticated assistance is P2; the interaction remains user-controlled.

# 50. Daily Production View
P1 summary of a single shooting day.

## Primary question
> **What do I need to know for this day?**

## Entry points
- Production > Daily View
- Shooting Day > Daily View

## Recommended layout
Top: date/day.

Summary cards:
Scenes
Cast
Locations
Estimated duration
Important notes

Call Sheet link is prominent.

## Visible controls
- Select day
- Open Scene
- Open Person
- Open Location
- Open Call Sheet
- Print/Export

## User actions and behavior
All values come from the selected Shooting Day and linked records. The page is mostly read/inspect rather than a second schedule editor.

## Visible states
- No day
- Ready day
- Day with issues
- Finalized

## Empty state
`Select or create a shooting day.`

## Errors and recovery
Source or document issues are shown as contextual warnings.

## Cross-module connections
Daily View connects schedule, cast, location and call sheet.

# 51. Call Sheets
Call Sheet workspace lists existing call sheets and provides creation entry points.

## Primary question
> **Which shooting day document do I need?**

## Entry points
- Global navigation > Call Sheets
- Project Home

## Recommended layout
List of call sheets sorted by shooting date/day number. Each row shows date, day number, status and stale/finalized state.

Top action: Create Call Sheet.

## Visible controls
- Create Call Sheet
- Open
- Search
- Filter by status
- Export
- Finalize

## User actions and behavior
Creating a call sheet requires a source Shooting Day. Existing sheets open in the Call Sheet Editor.

## Visible states
- None
- Draft
- Source changed
- Ready
- Finalized
- Superseded

## Empty state
`Create a call sheet from a shooting day.`

## Errors and recovery
If a source day is unavailable, the app explains it without deleting the historical call sheet.

## Cross-module connections
This is the collection view for UX-52/53; the actual editor is specified below.

# 52. Call Sheet Editor
Document-first daily production sheet.

## Primary question
> **What does everyone need to know for this day?**

## Entry points
- Call Sheets > Create/Open
- Shooting Day > Create Call Sheet

## Recommended layout
Document body contains:

Production header
Crew call
Cast calls
Scenes
Location/address
Practical notes
Emergency contacts
Optional weather/attachments/special notes

Side panel: Source Day + source/stale status.

## Visible controls
- Create from day
- Edit call times
- Edit notes
- Meeting point
- Attachments
- Refresh
- Finalize
- Export

## User actions and behavior
Source data prefills scenes, cast and location. User edits call times and practical details directly.

Schedule edits make the document stale. Refresh opens a preview of changes before applying them.

## Visible states
- None
- Draft
- Source Changed
- Ready
- Finalized
- Superseded

## Empty state
Missing information appears as blank editable areas where allowed rather than blocking the entire document.

## Errors and recovery
Refresh ambiguity or source mismatch must open a review rather than overwrite the call sheet.

## Cross-module connections
Editing the call sheet never reverse-syncs to the schedule.

# 53. Call Sheet Finalize and Export
Create a stable day-of snapshot.

## Primary question
> **Is this the version I am actually sending?**

## Entry points
- Call Sheet > Finalize
- Call Sheet > Export

## Recommended layout
Small preview + settings area. Status is visible above the document.

Finalize action is clearly separate from Export.

## Visible controls
- Refresh
- Finalize
- Export PDF
- Print
- Cancel

## User actions and behavior
Finalize freezes the current document state as a historical version. Export produces the PDF snapshot. Later source changes can mark the sheet stale/superseded without changing the issued document.

## Visible states
- Ready
- Finalizing
- Finalized
- Superseded

## Empty state
If a required source value is missing, return to editor with the missing field highlighted.

## Errors and recovery
Export/finalization failure preserves the existing call-sheet draft.

## Cross-module connections
The issued PDF can be shared through the user's own communications channel.

# 54. Production Supporting Workspace
Production is a grouped area containing the approved small-team production tools.

## Primary question
> **What production information do I need right now?**

## Entry points
- Production navigation

## Recommended layout
Use compact tabs/subnavigation for:

Catalog
Locations
Cast & Crew
Moodboards
Storyboards
Shot Lists
Shooting Schedule
Daily View
Sides
Reports
Budget
Notes/Tasks

Only one sub-workspace is primary at a time.

## Visible controls
- Choose workspace
- Search current workspace
- Add
- Open
- Export

## User actions and behavior
Switching tabs changes the active production view without duplicating or merging data.

The navigation itself should remain compact. If a short film uses only locations and a shot list, the other tools do not need prominent empty cards.

## Visible states
- Production open
- Sub-workspace selected
- Empty sub-workspace

## Empty state
Each sub-workspace owns its empty state.

## Errors and recovery
Errors remain contained within the selected workspace.

## Cross-module connections
All production sub-workspaces connect to existing objects but keep one-page/one-job behavior.

# 55. Reports
Basic production reports are P1 snapshots derived from source data.

## Primary question
> **What concise production information do I need to see or send?**

## Entry points
- Production > Reports

## Recommended layout
Report picker at top. Preview below.

Core options:
Scene Report
Location Report
Cast Scene Report
Prop Report
Schedule Report
Breakdown Completeness

## Visible controls
- Select report
- Filter
- Preview
- Export
- Print

## User actions and behavior
Reports are read-only snapshots of current project information. Filters are minimal and understandable.

Report data is never edited directly in the report view.

## Visible states
- Report selected
- Filtering
- No results
- Preview
- Exporting

## Empty state
No results should say `No matching data.` and keep filter controls visible.

## Errors and recovery
If a source is temporarily unavailable, keep the report selection and retry without mutating project data.

## Cross-module connections
P2 additional reports may later use the same screen pattern; no separate reporting application is created.

# 56. Sides
Sides are a P1 production document containing a selected scene set from the chosen script source.

## Primary question
> **What script pages/scenes does this team need for this shooting day?**

## Entry points
- Shooting Day > Sides
- Production > Sides

## Recommended layout
Small setup area: source shooting day, selected scenes and optional cover/revision choice. Main body is clean page preview.

## Visible controls
- Select day
- Select scenes
- Include revision
- Optional cover
- Preview
- Export

## User actions and behavior
Generate sides from a selected shooting day or chosen scene set. The side document is a snapshot; it does not change schedule or screenplay.

## Visible states
- No day
- Selected scenes
- Preview
- Export

## Empty state
Explain the selected source and scene set before export.

## Errors and recovery
If a source draft cannot be resolved, ask the user to select a valid source instead of producing an ambiguous document.

## Cross-module connections
Sides reference screenplay and shooting schedule but do not become a separate production planning database.

# 57. Lightweight Budget Snapshot
The budget feature is intentionally small and P1.

## Primary question
> **How much are we roughly planning to spend?**

## Entry points
- Production > Budget

## Recommended layout
Compact budget screen:

Project estimate at top.

Below: category list with line items and totals.

Optional contingency line.

No accounting dashboard, payroll or financial workflow.

## Visible controls
- Set total/estimate
- Add category
- Add line
- Amount
- Contingency
- Edit
- Export

## User actions and behavior
A budget line requires description and amount. Category totals roll up to a project snapshot. The user can edit/delete lines.

The interface should state that this is a planning estimate, not formal accounting.

## Visible states
- No budget
- Draft estimate
- Populated
- Editing

## Empty state
Empty state: `Use a simple estimate for planning. Formal accounting is outside OpenFrame.`

## Errors and recovery
Reject invalid negative amounts and malformed values. Preserve existing valid lines if one entry fails.

## Cross-module connections
Budget is independent support information. It does not change schedule, cast or screenplay unless the user manually uses the information elsewhere.

# 58. Project Notes
Project Notes are freeform supporting notes outside the Idea Vault and other specific workspaces.

## Primary question
> **What do I need to remember that does not belong anywhere else?**

## Entry points
- Production > Notes/Tasks
- Project Home notes

## Recommended layout
List of notes with title, short preview and last modified.

Selecting a note opens a detail drawer with title, body, attachment and optional related object.

## Visible controls
- New Note
- Search
- Edit
- Attach
- Relate
- Print/Export
- Delete

## User actions and behavior
Notes may contain meeting notes, decisions, reminders or production observations. They do not appear in formal documents unless explicitly exported.

## Visible states
- Empty
- Selected
- Editing
- Saved
- Search

## Empty state
`Add a note for something that doesn't naturally belong in Idea Vault or another workspace.`

## Errors and recovery
Save failures preserve the note text. Delete uses recoverable behavior.

## Cross-module connections
Optional related object creates a reference only; editing a note does not mutate the related object.

# 59. Lightweight Tasks
Tasks are optional reminders rather than a project-management suite.

## Primary question
> **What action do I need to remember?**

## Entry points
- Production > Tasks
- Project Home Tasks

## Recommended layout
Compact list with columns/fields for title, status, due date if present, owner if present, related object if present.

## Visible controls
- New Task
- Mark Done
- Reopen
- Delete
- Filter
- Open Related

## User actions and behavior
A task requires only a title. Due date, owner and related object are optional. Completing a task changes only task status.

There is no dependency graph, Gantt, time tracking or sprint board.

## Visible states
- No tasks
- Open
- Done
- Filtered

## Empty state
`Add a small task only when you need to remember an action.`

## Errors and recovery
Deleted tasks are recoverable as defined by FSD.

## Cross-module connections
Related object links open the source workspace without changing its state.

# 60. Activity History
Activity History records meaningful project events without recording every keystroke.

## Primary question
> **What important actions have happened in this project?**

## Entry points
- Project Home > Activity
- Production > Activity

## Recommended layout
Read-only chronological list.

Each item shows actor, action, date/time and affected object where meaningful.

## Visible controls
- Filter by type
- Open affected object
- Scroll
- Search optional

## User actions and behavior
Record meaningful events such as project created, draft created/locked, scene card moved, breakdown source changed, catalog created, shooting day created, call sheet finalized, exchange import/export and collaboration session start/end.

Ordinary typing is not shown as an activity stream event.

## Visible states
- No history
- History populated
- Filtered

## Empty state
Empty project may simply show `No major activity yet.`

## Errors and recovery
History is read-only; there is no edit/delete control for normal entries.

## Cross-module connections
History can link to source objects but must never become a second project state store.

# 61. Template Browser
Templates are P1 and should accelerate common starts without forcing setup.

## Primary question
> **Would a saved starting point help me begin faster?**

## Entry points
- New Project > From Template
- Project/template action

## Recommended layout
Compact template cards with type and preview. Built-in templates and User Templates can be separated.

## Visible controls
- Preview
- Use Template
- Create Template
- Rename
- Delete

## User actions and behavior
Applying a template creates independent project content. Template edits do not modify projects already created from it.

Only a small number of useful templates should be prominent.

## Visible states
- No templates
- Built-in
- User-created
- Preview

## Empty state
A standard blank project path always remains available.

## Errors and recovery
Invalid template content falls back to the standard project creation workflow rather than blocking project creation.

## Cross-module connections
Template use is optional and never a prerequisite for core filmmaking workflows.

# 62. Files
Project Files is a lightweight file cabinet for miscellaneous project documents.

## Primary question
> **What files belong with this film?**

## Entry points
- Shell > Files
- Project Home > Files

## Recommended layout
List or grid with filename, type, last modified and lightweight folder context. Preview/detail is available for selected files.

## Visible controls
- Add File
- Open
- Move
- Rename
- Export/Copy
- Delete
- Search

## User actions and behavior
Files may include pitch decks, research documents, permission scans, externally received schedules and artwork.

Users can drag supported files into the page.

## Visible states
- Empty
- Populated
- Selected
- Unavailable file

## Empty state
`Keep project documents and attachments here.`

## Errors and recovery
Unavailable/missing files show an explicit state. Deletion is recoverable where defined.

## Cross-module connections
Files are not an asset-management system and should not duplicate the Idea Vault without a deliberate action.

# 63. Permissions
Simple project-level access control supports small-team collaboration.

## Primary question
> **Who can see, edit or comment on this project?**

## Entry points
- Project access controls
- Exchange/LAN setup

## Recommended layout
Compact list of people with role selector:

Owner
Editor
Commenter
Viewer
Export-only

Private areas show a separate visibility indicator.

## Visible controls
- Add person
- Change role
- Remove access
- Review

## User actions and behavior
Owner has full project control. Editor can edit authorized project content. Commenter can review/comment. Viewer is read-only. Export-only can create permitted snapshots/packages but cannot enter live editing sessions.

A person can hold multiple roles where the model allows it.

## Visible states
- Owner only
- Collaborators
- Role change
- Removed

## Empty state
Owner-only state should remain simple: `You are the owner.`

## Errors and recovery
Reducing permissions should use explicit confirmation. Invalid permission state does not mutate access.

## Cross-module connections
Permissions affect exchange and LAN session behavior but do not introduce enterprise identity management.

# 64. Script Review Exchange
Portable review packages are the primary remote review workflow.

## Primary question
> **How do I send this draft to someone for review and safely bring their notes back?**

## Entry points
- Screenplay > Share/Export Review Package

## Recommended layout
Medium/large export dialog.

Top: `Script Review Package`

Source Draft
Scope: Full Script / Selected Scenes
Include comments: Yes/No
Include attachments: Optional

Preview before export.

## Visible controls
- Choose draft
- Select scenes
- Include comments
- Include attachments
- Preview
- Export

## User actions and behavior
Export creates a snapshot package. The user sends it using their normal communication channel.

The reviewer imports the package, reads/comments locally, and exports a response package. The original author imports the response, sees mapping status and selects what to apply.

## Visible states
- Preparing
- Ready
- Exported
- Response received
- Mapping preview
- Applied

## Empty state
Empty selection should clearly say a review package needs a draft and optionally a scene selection.

## Errors and recovery
Package generation failure leaves the project unchanged. Invalid response packages go to validation rather than mutating the current project.

## Cross-module connections
Comment mapping uses object identity/text/context rules. No blind screenplay overwrite.

# 65. Exchange Package Import Preview
All exchange imports pass through validation and a change preview.

## Primary question
> **What exactly will this imported package change?**

## Entry points
- Files > Import Exchange
- Workspace > Import Exchange

## Recommended layout
Large modal with:

Package Type
Source Project
Source Version
Compatibility
Changes by object
Ambiguous mappings
Unmapped notes

Footer actions: Import Selected, Accept All Safe, Cancel.

## Visible controls
- Open package
- Review differences
- Select items
- Accept all safe mappings
- Cancel
- Open Review Queue

## User actions and behavior
Validation happens before mutation. Safe identity mappings can be selected. Ambiguous mappings go to Review Queue. No safe mapping remains as an Unmapped Review Note. Current project content is never silently overwritten.

## Visible states
- Valid
- Stale
- Conflict
- Ambiguous
- Unmapped
- Rejected

## Empty state
A rejected package returns the user to the workspace with no changes.

## Errors and recovery
Validation failure means zero project mutation.

## Cross-module connections
This surface supports Story, Script, Breakdown, Shot, Schedule and Call Sheet package types using the same interaction pattern.

# 66. Local Network Collaboration — Host
Optional LAN session allows real-time collaboration while project remains local.

## Primary question
> **Who is joining and what will I share?**

## Entry points
- Project > Start Session

## Recommended layout
Setup dialog:

Project
Shared scope: Entire Project / Story / Screenplay / Production
Participant role list
Join information

Active session becomes a small shell bar.

## Visible controls
- Start
- Choose scope
- Assign roles
- Copy join information
- End Session

## User actions and behavior
Host remains working on the local project. Scope controls access.

If sharing Story only, private Screenplay/Production material remains unavailable to the participant.

## Visible states
- Starting
- Active
- Waiting
- Participant joined
- Participant disconnected
- Ending

## Empty state
No participants is a valid session state; host can continue or end.

## Errors and recovery
Failure to start session never blocks solo local work.

## Cross-module connections
No project data is published to OpenFrame cloud.

# 67. Local Network Collaboration — Participant
Participant view is focused on the shared workspace.

## Primary question
> **What am I allowed to work on?**

## Entry points
- Join session

## Recommended layout
Join dialog asks for identity/name and displays assigned role + shared scope. After joining, normal workspace opens with a small session indicator.

## Visible controls
- Join
- Leave
- View session scope

## User actions and behavior
The participant sees only the shared scope. Export-only users cannot enter a live editing session.

## Visible states
- Joining
- Connected
- Disconnected
- Rejoining

## Empty state
A participant can continue with local work if disconnected, subject to project conflict rules.

## Errors and recovery
If joining fails, no local project content is altered.

## Cross-module connections
Presence and editing behavior follow the collaboration contract.

# 68. Local Collaboration Presence and Conflict
This surface handles visible presence and advanced conflict decisions.

## Primary question
> **Who is working here, and did two people change the same thing?**

## Entry points
- Active LAN session
- Object edited concurrently

## Recommended layout
Small presence avatar/name indicators near active objects. Conflict opens a focused comparison panel:

Your version
Incoming version
Context

Actions: Keep Mine, Use Incoming, Review Differences.

## Visible controls
- View participant
- Review conflict
- Keep Mine
- Use Incoming
- Review Differences
- Retry/Rejoin

## User actions and behavior
Soft locks may show who is currently editing an object, but never permanently lock a user out.

Conflicting edits are not silently discarded.

## Visible states
- No conflict
- Soft lock
- Conflict
- Resolved
- Disconnected

## Empty state
No presence can simply show the session as active without extra indicators.

## Errors and recovery
Disconnect retains local work. Rejoin presents unresolved conflicts rather than silently choosing one version.

## Cross-module connections
Basic LAN collaboration is P0; advanced conflict handling is P1.

# 69. Offline / Save / Recovery
Offline is a normal state; local project ownership must remain clear.

## Primary question
> **Is my work safely stored locally?**

## Entry points
- Bottom status bar
- Connection change
- Save error
- Project recovery

## Recommended layout
Bottom status communicates Offline/Saved/Saving/Save Error.

Clicking it opens a small status/recovery panel rather than taking the user away from their workspace.

## Visible controls
- View status
- Retry save
- Open recovery
- Create backup
- Open project location

## User actions and behavior
When offline, core workflows remain enabled: idea editing, story boards, screenplay, breakdown, schedule, call sheet and export.

AI may be unavailable.

A save error keeps current content visible and offers retry/recovery.

## Visible states
- Online
- Offline
- Saving offline
- Save error
- Recovery available

## Empty state
`Offline — your project is stored locally.`

## Errors and recovery
Never block core work merely because network is unavailable. If local storage fails, show a serious recovery state without claiming cloud backup exists.

## Cross-module connections
This is a cross-cutting state rather than a separate project-management page.

# 70. Full Project Package / Portability
Portable project files allow movement between machines and safe backup.

## Primary question
> **How do I take the whole film project with me?**

## Entry points
- Project > Export Full Project
- Backup

## Recommended layout
Package export dialog shows workspaces and selected files. A brief summary states what will be copied.

## Visible controls
- Select included content
- Export
- Import as new project
- Import as copy

## User actions and behavior
Import creates a new local project/copy and never silently replaces the current project.

## Visible states
- Selecting
- Preparing
- Complete
- Validation
- Importing

## Empty state
Show included content before export.

## Errors and recovery
If package validation fails, abort before project mutation.

## Cross-module connections
This is the highest-level user-owned portability workflow.

# 71. AI Assistant

The AI assistant is an optional system-wide natural-language assistant and command surface. It is optional and must never become an autonomous project editor.

## Primary question

> **What do I want OpenFrame to find, explain, prepare, or help me do?**

## Entry points

- Shell AI action
- Contextual AI action
- Global command/search area
- Workspace-specific AI action

## Recommended layout

Right-side panel:

```text
AI Assistant
────────────────────────
Scope: [Current Scene ▾]

Using:
Scene 24 + Characters + Breakdown

Conversation
...

Answer / Result
...

For a mutation:
Proposed Changes
Affected objects
Impact
Conflicts
[Review Changes] [Apply] [Cancel]
```

The assistant should feel like part of the application shell, not a separate chatbot product.

## Context choices

At minimum:

- Current Selection
- Current Scene
- Current Screenplay
- Specific Draft
- Story
- Selected Idea Vault Items
- Production
- Shooting Day
- Call Sheet
- Whole Project

## User actions and behavior

Read-only, calculation and navigation requests may execute directly when explicitly requested.

Examples:

- “How many characters are in Draft 8?”
- “How many locations are in the screenplay?”
- “Which scenes contain Ravi?”
- “Open Scene 42.”
- “Show the unscheduled scenes.”
- “Compare Draft 7 and Draft 8.”

Exact counts and statistics should be presented as application-derived results, not model estimates.

For project-changing requests:

1. Resolve target and scope.
2. Check permissions/state.
3. Prepare exact Change Set.
4. Show preview.
5. Require explicit acceptance.
6. Apply through the normal OpenFrame action.
7. Expose undo/activity as applicable.

The AI must never silently rewrite screenplay/story/production/schedule data.

## Global command behavior

The user can issue natural-language commands without manually navigating to the destination workspace first.

Examples:

> `Open Scene 42.`

> `Find every project file mentioning railway station.`

> `Prepare a breakdown for Scene 24.`

> `Rename Ravi to Raghav across structured references.`

The command layer routes to existing OpenFrame capabilities rather than creating a separate action system.

## Batch and rename preview

For batch or project-wide operations, the preview should show:

- affected object count;
- affected modules;
- structured references;
- excluded raw text;
- locked/unauthorized targets;
- conflicts;
- resulting project state.

Structured Character/name references must be visually separated from arbitrary dialogue/action text.

## Visible controls

- Ask / Command
- Select scope
- Copy
- Review Changes
- Accept / Apply
- Reject / Cancel
- Retry

## Visible states

- AI off
- Ready
- Interpreting
- Retrieving
- Generating
- Answer
- Mutation preview
- Awaiting approval
- Applying
- Applied
- Rejected
- Stale
- Conflict
- Failed
- External disclosure
- Unavailable

## Errors and recovery

AI failure leaves local content unchanged.

A stale proposal must return to review/revalidation rather than applying against a changed project.

Permission failures explain that the requested action is not available to the current user.

## Scope/provenance

When useful, show:

`Using: Draft 8 + Characters + Breakdown`

or:

`Using: Whole Project`

For exact results, the UI may expose source/version context so the user can understand what was counted.

## No autonomous behavior

The interface must never imply that the AI is monitoring and changing the project in the background.

Suggestions remain suggestions until the user accepts them.

# 72. AI External Disclosure
External AI requests require clear data-disclosure UI.

## Primary question
> **What information is being sent to an external AI provider?**

## Entry points
- AI request using external provider

## Recommended layout
Before sending, show a compact disclosure panel stating the selected context and that content will be sent to the configured provider.

## Visible controls
- Review context
- Continue
- Cancel

## User actions and behavior
User can cancel before transmission. The disclosure is specific to the request context, not a generic privacy notice.

## Visible states
- Awaiting consent
- Sending
- Canceled

## Empty state
The disclosure must be readable without opening settings.

## Errors and recovery
If the request cannot be sent, the project is unchanged.

## Cross-module connections
This is a UX presentation of the FSD AI external data-handling flow.

# 73. Global Empty, Loading, Error and Recovery States

## 73.1 Empty state grammar
Every empty state uses three parts:

1. What this area is.
2. What the user can do.
3. One primary action.

Example:

`Your Story Board is empty.`

`Scene Cards are short reminders of what happens in each scene. Drag them until the story works.`

`+ Add Scene`

Do not add multiple promotional cards, tutorial videos or unrelated links.

## 73.2 Loading state
Loading should preserve the page structure. Use lightweight placeholders for lists/cards or a compact `Loading…` state in the active pane.

The application should not replace the entire desktop with a generic spinner for local operations.

## 73.3 Save error
The user should see:

`Save error`

followed by:

`Your current work is still open. Retry save or open Recovery.`

The error should not imply that an internet connection is necessary.

## 73.4 External file unavailable
Use a small unavailable badge and provide an action such as `Relink` or `Open Location` when supported. Do not remove the item because the source file temporarily disappeared.

## 73.5 Import failure
Show the source filename, stage of failure and whether any project changes were made. For supported import safety, the correct normal result is:

`Import failed. Your current project was not changed.`

## 73.6 Unsaved switch
When leaving a project during a save problem, the dialog should offer:

`Stay`
`Retry Save`
`Close Without Leaving` only when the user explicitly accepts the consequence

The application must not use ambiguous button labels such as `OK` and `Cancel` for this decision.

## 73.7 Recovery offer
If a crash/recovery point exists on reopen, show:

`We found a recent recovery state for this project.`

Then:

`Open Recovery`
`Keep Saved Version`

Do not silently select one version.

# 74. Object Detail Drawer Standards

## 74.1 Drawer anatomy
Default drawer:

Header:
- object name/title
- object type
- close

Body:
- primary editable fields
- optional fields under Add Details
- related object links
- attachments
- comments where supported

Footer only when needed:
- Save
- Cancel
- Delete/Archive

For simple objects, edits may save automatically; the drawer should not force a form workflow.

## 74.2 Drawer opening behavior
Opening a drawer should preserve the underlying workspace scroll position and selection. Closing returns the user to exactly the same place on the board/list/editor.

## 74.3 Drawer width
Use a consistent medium detail width. Very long screenplay or document content belongs in the main workspace, not inside a cramped drawer.

## 74.4 Unsaved drawer changes
If the drawer contains a save-required edit, Escape/Close should follow the project's normal save/cancel rule. Small inline edits can commit on blur where appropriate.

## 74.5 Related links
Related object links are concise. For a scene, use:

`Open Screenplay`
`Open Breakdown`
`Shots (4)`
`Storyboard (3)`
`Schedule — Day 7`

Clicking a related link navigates; it does not open an enormous nested view inside the drawer.

# 75. Global Contextual Navigation Rules

## 75.1 Open in… pattern
Every major object should expose `Open in…` where the FSD/PRD provides a meaningful destination.

For a screenplay scene:
- Story Board
- Screenplay
- Breakdown
- Shot List
- Storyboard
- Schedule

For a Catalog Location:
- Locations
- Scenes using it
- Schedule where used

For a Shooting Day:
- Schedule
- Daily View
- Call Sheet
- Sides

## 75.2 Return behavior
When the user opens another workspace from a Scene Hub or related link, the destination should focus the same scene/object.

A visible breadcrumb or compact back action should help return to the origin when useful.

## 75.3 Do not duplicate navigation
Do not create a second permanent navigation tree for every context. `Open in…` is contextual and temporary.

# 76. Context Menus — Canonical UX

|Object|Context menu order|
|---|---|
|Scene Card|Open · Duplicate · Move · Send to Parking Lot · Convert to Screenplay Scene · Add Comment · Export · Delete|
|Screenplay Scene|Open in Story Board · Break down · Create Shot List · Create Storyboard · Add Comment · Copy Scene · Export Scene|
|Beat|Open/Edit · Convert to Scene · Duplicate · Move · Park · Add Comment · Delete|
|Sequence|Open · Rename · Move · Add Scene · Add Beat · Add Comment · Delete|
|Act|Rename · Move · Add Sequence · Add Scene · Add Comment · Delete|
|Idea Vault Item|Open · Rename · Move · Pin · Copy to Project · Send to Story · Export · Delete|
|Shot|Open · Duplicate · Reorder/Move · Attach Storyboard · Add Comment · Export · Delete|
|Storyboard Panel|Open · Move · Link Shot · Add Comment · Export · Delete|
|Catalog Item|Open · Rename · Open Scenes · Archive · Add Comment · Delete|
|Location|Open · Edit · Open Scenes · Add Comment · Archive/Delete|
|Call Sheet|Open · Refresh · Finalize · Export · Add Comment · Delete|
## 76.1 Context-menu rule
Only actions that make sense for the selected object appear. Menu order should privilege frequent actions. Delete remains visually separated from normal actions.

# 77. Multi-Select and Batch UX

## 77.1 Where multi-select is supported
Core multi-select applies to:

- Story Board cards
- Idea Vault items
- selected production items where batch operations are explicitly useful
- exchange package selections
- scene selections for documents such as sides

## 77.2 Selection toolbar
When multiple objects are selected, a compact toolbar appears near the top of the workspace.

It should show only valid shared actions, such as Move, Park, Export or Delete.

If the selection contains different object types, the toolbar can offer only the intersection of valid actions.

## 77.3 Clear selection
Click blank space or press Escape to clear selection where appropriate. Do not require a dedicated Close Selection button.

## 77.4 Batch destructive action
Deleting multiple objects requires one confirmation that explains the count and any downstream relationship warning. It should not open one dialog per object.

# 78. Keyboard Shortcut Philosophy

## 78.1 Core shortcuts
Use familiar desktop conventions:

- Ctrl/Cmd+Z — Undo
- Ctrl/Cmd+Shift+Z — Redo
- Ctrl/Cmd+F — Contextual search
- Ctrl/Cmd+S — Save
- Ctrl/Cmd+P — Print/Preview where appropriate
- Ctrl/Cmd+N — New Project from Home or contextually new object
- Ctrl/Cmd+O — Open Project/Document
- Escape — close current drawer/dialog/panel or clear transient mode

## 78.2 Story Board shortcuts
Where implemented:

- Arrow keys — move focus through cards
- Enter — open/edit selected card
- Delete — invoke delete behavior
- Space — selection toggle if practical
- Modifier + click — multi-select

## 78.3 Screenplay shortcuts
Use standard professional writing-editor conventions for element switching where possible. Exact key bindings can be documented in the product's command list, but the UX must always provide an on-screen alternative.

## 78.4 Shortcut discoverability
Command tooltips and menus can show shortcuts. Do not display every shortcut permanently; the user discovers them naturally.

# 79. Idea Vault Visual Behavior Matrix

|Item type|Collapsed presentation|Expanded presentation|Primary action|Special UX rule|
|---|---|---|---|---|
|Text note|Text preview card|Full editable note|Edit|Title may remain blank|
|Image|Thumbnail + optional caption|Large preview + metadata|Open|Original image remains primary source|
|URL|Title/URL preview|URL + note + preview|Open|Preview metadata is secondary to URL|
|PDF|File card + page/preview indicator|Document preview + file info|Open|External open available|
|Document|Filename/type card|Preview/open externally|Open|Do not require semantic classification|
|Audio/voice note|Audio card + duration|Playback + optional transcription|Play|Audio remains authoritative|
|Video|Video thumbnail + duration|Local preview if available|Play|Not a video-editing surface|
|Handwritten/sketch|Thumbnail|Large preview|Open|No transcription required|
|Quote|Quoted text preview|Full quote + note/source|Edit|Source is optional|
|Screenshot|Thumbnail|Large preview + note|Open|Treat as ordinary visual reference|
|Other file|Filename/type card|System preview/open if possible|Open|Unsupported preview must not mean unsupported storage|
## 79.1 Drag-in behavior
Dragging several mixed files into the Vault should show a compact import confirmation only if necessary to communicate file count or an error. Supported items should be accepted together. The user should not be asked to choose a semantic category for each item.

## 79.2 Visual hierarchy
Images/video receive larger previews. Text notes receive text-first cards. Files receive file-type cues. The Vault should therefore feel naturally visual rather than forcing every item into one identical rectangle.

# 80. Story Board Visual Rules

## 80.1 Act visual treatment
Acts create broad visual bands or columns. They should be visually stronger than sequences but not so large that a three-act short film consumes the entire screen.

## 80.2 Sequence visual treatment
Sequence headers are compact labels with a visible child count. A sequence should look like a container, not like a special workflow requiring forms.

## 80.3 Scene Card geometry
Scene Cards are small horizontal rectangles. Their primary text area should support two or three comfortable lines before truncation. The card can expand through double click.

The card should remain readable when many scenes are visible. Avoid showing ten icons on the card.

## 80.4 Beat Card geometry
Beat Cards are visually lighter than Scene Cards because they represent smaller thoughts. They should be draggable and easy to create inline.

## 80.5 Parking Lot
Parking Lot uses a visually different background/border treatment so cards clearly appear outside the active narrative order. It remains close enough to the board that moving a card back is obvious.

## 80.6 Zoom
Board zoom changes card presentation density but never changes stored content. At low zoom, cards can show one-line summaries. At higher zoom, cards show more text. The board remains a reference surface; zoom is a viewing aid.

# 81. Scene Card Detailed Interaction Map

|Action|User gesture|Immediate visual response|Data effect|Downstream effect|
|---|---|---|---|---|
|Create|Click +Scene|New compact card focused|Creates new identity|None until build/explicit use|
|Edit description|Click/Enter card|Inline editing|Updates card content|Existing screenplay unchanged|
|Edit heading|Open detail > Heading|Field focus|Updates card heading|Only future build uses change|
|Move within sequence|Drag|Insertion line|Order changes|Screenplay not silently changed|
|Move sequence|Drag to another sequence|Destination highlight|Parent sequence changes|Identity/attachments/comments retained|
|Duplicate|Context menu|New selected card|Creates new identity|No screenplay identity copied|
|Park|Drag to Parking Lot|Card leaves active outline|Marked parked|Screenplay unchanged|
|Restore|Drag from Parking Lot|Insertion point|Active order restored|Screenplay unchanged|
|Comment|Select > Comment|Thread opens|Comment created|No script mutation|
|Delete|Context menu > Delete|Confirmation/recovery|Card recoverably deleted|Linked screenplay scene remains|
|Build|Build Screenplay|Preview opens|No mutation until confirm|Creates screenplay draft|
## 81.1 Accidental drag protection
A drag gesture is only committed when released over a valid destination. Releasing outside valid targets returns the card to its original location. There should be no half-move state visible to the user.

## 81.2 Long descriptions
The collapsed card truncates visually. The stored description remains unchanged. Expanded view allows full editing and wrapping.

# 82. Screenplay Editor Visual Rules

## 82.1 Page-first composition
The screenplay page should be centered with a readable left/right margin around it. Browser-like chrome and utility widgets should not overpower the writing page.

## 82.2 Scene separator
Scene headings should be visually recognizable without becoming oversized. A compact scene marker can help the navigator align with the page.

## 82.3 Current cursor context
The editor can show a subtle element-type indicator for the active paragraph. This is useful for understanding whether the cursor is in Action, Dialogue, Character or other screenplay elements.

## 82.4 Selection and comments
Selecting text should preserve normal copy/paste behavior. A nearby Comment affordance appears without covering the selected text.

Comment anchors visually attach to the text region or appear in the comments panel; they do not rewrite the content.

## 82.5 Locked script presentation
The current draft status should be visible but not visually dominate. `Locked Shooting Draft` can appear as a badge in the draft selector and as a small contextual banner when attempting to edit.

## 82.6 Revision presentation
Production revision state should be visible near the draft name. Revision color can appear in the script/print preview where supported, but normal writing should not be permanently tinted.

# 83. Breakdown Visual Rules

## 83.1 Three-pane balance
Breakdown needs enough width for the screenplay text and enough room for production items. The scene list is narrow; the script pane is dominant; the breakdown pane is compact but useful.

## 83.2 Category grouping
Default categories are collapsible groups. Categories with no items can remain collapsed or hidden behind a small count to avoid a giant empty checklist.

## 83.3 Suggested vs confirmed
Suggested elements must have a visibly different state from confirmed production elements.

Suggested:
`Suggested — Red Folder`

Confirmed:
`Red Folder`

This is essential because suggestion is not decision.

## 83.4 Highlighted source text
When a breakdown item came from highlighted screenplay text, the production panel can show a small contextual quote/reference. It should never require the user to navigate away from the script to remember where the item came from.

## 83.5 Completion
A scene marked Breakdown Complete receives a compact check/status. It should still be possible to reopen and change it.

# 84. Production Catalog and Detail UX

## 84.1 Catalog list
Use compact rows or cards with name, category and status. A searchable list is preferable to a large spreadsheet for the target audience.

## 84.2 Item detail
Detail drawer:

Name
Category
Status
Image
Description
Notes
Scenes using item

No financial or procurement fields.

## 84.3 Existing match
When a breakdown suggestion resembles an existing catalog item, the UI should say:

`Possible existing item: Red Folder`

with choices:

`Use existing`
`Create new`

If several plausible items exist, show the candidates and let the user choose.

## 84.4 Archive
Archived items are visually muted and excluded from normal creation suggestions where appropriate, but historical scene/document relationships remain readable.

# 85. Location UX

## 85.1 Location list
Rows/cards show a representative image, name, status and scene count. Status is the most useful scanning signal.

## 85.2 Location detail
Detail structure:

Header: name + status
Photos
Address/Area
Contact
Practical Notes
Scenes

Practical notes can use small labeled lines such as Parking, Noise, Permission, Access, Power and Travel.

## 85.3 Photo workflow
Add Photo accepts one or multiple images. Photos can be reordered. The first/selected image can act as the main thumbnail.

## 85.4 Confirm/reject
Status buttons are simple and reversible. Confirming a location does not automatically assign it to every scene; scene associations remain explicit.

# 86. Cast & Crew UX

## 86.1 Cast list
Rows show:

Person
Character
Photo
Availability note
Scene count

Avoid salary or contract fields.

## 86.2 Character assignment
Assign Actor opens a compact chooser of existing characters. Creating a new character from inside the chooser is allowed only if it remains the same basic Character object behavior.

## 86.3 Crew list
Rows show:

Person
Role
Department
Contact

Optional notes remain in detail.

## 86.4 Availability
Availability is intentionally lightweight. The user can record useful availability notes without opening a calendar/HR subsystem.

# 87. Moodboard and Storyboard Visual Planning UX

## 87.1 Moodboard canvas
The moodboard is a freeform visual canvas. Items can be placed with flexible positioning and optional size changes. The user should feel like pinning references on a wall.

## 87.2 Storyboard panel
Storyboard is more sequential than Moodboard. Panels appear in order left-to-right or top-to-bottom. A selected panel opens its description and optional shot link.

## 87.3 Import vs draw
The Add Panel action offers:

`Import Image`
`Draw/Sketch`
`Empty Panel`

This keeps basic storyboarding accessible without pretending to be a full illustration application.

## 87.4 Panel linking
If a panel is linked to a shot, show a small `Shot 24B` badge. Clicking it navigates to that shot. Removing the link does not delete either object.

# 88. Shot List UX

## 88.1 Scene grouping
Shot List should be visibly grouped by scene. Scene selection is persistent while the user adds/reorders shots.

## 88.2 Shot card
Collapsed shot card:

`24A — Wide — Arjun enters station`

Optional tiny chips:
`Static`
`WS`

Expanded drawer contains technical fields.

## 88.3 Shot numbering
Shot numbering is generated from scene order within the shot list. The user never has to renumber shots manually after reordering.

## 88.4 Coverage organization
The user can group shots visually by their order of capture/coverage. The software does not force a coverage methodology.

# 89. Schedule / Stripboard UX Detail

## 89.1 Overall composition
The stripboard is the most horizontally dense workspace. Use clear columns or lanes for shooting days.

Suggested composition:

```text
UNSCHEDULED
────────────────────────────────────────────
Scene 12 | Scene 18 | Scene 27 | ...

SHOOT DAY 1
────────────────────────────────────────────
[12] [15] [9]   Meal   [21]

SHOOT DAY 2
────────────────────────────────────────────
[18] [22] [23]
```

## 89.2 Strip anatomy
Each strip shows only the practical minimum:

Scene number
INT/EXT
Location
Day/Night
Page count
Synopsis

Small icons/badges can indicate cast or breakdown presence.

## 89.3 Drag feedback
Dragging a strip shows a clear insertion line within a day and a destination highlight across day containers. The user should understand where the scene will land before releasing.

## 89.4 Unscheduled pool
The Unscheduled area remains visible while planning. Once a scene is scheduled it disappears from the pool.

Removing a scene from a day returns it to Unscheduled rather than deleting the scene from the screenplay.

## 89.5 Day markers
Break markers use compact horizontal rows or inline separators so they are visually distinct from scenes.

Examples:
`MEAL`
`TRAVEL`
`COMPANY MOVE`
`NOTE`

## 89.6 Day date
The shooting day header shows `Day 4 — 14 June 2027` where a date exists. The user can still work with a day before a date is fully planned if the FSD allows the state.

# 90. Schedule Conflict and Suggestion UX

## 90.1 Warning language
Warnings should be specific:

`Potential actor conflict: Arjun is needed at two locations on Shoot Day 3.`

`Potential location conflict: scenes require two locations at overlapping times.`

`Estimated day duration exceeds the target.`

## 90.2 Warning placement
Warnings appear near the affected day or scene. Do not open a large dialog for every warning.

## 90.3 Acknowledge/keep
The user can choose to keep the schedule. If the warning remains, a small acknowledged indicator can appear. No auto-resolution occurs.

## 90.4 Grouping suggestion
Suggestion panel explains the reason first:

`Scenes 12, 18 and 21 share the same location.`

Then:

`Consider grouping them to reduce location changes.`

The user manually moves scenes or explicitly applies a supported suggestion.

# 91. Call Sheet Document UX

## 91.1 Paper hierarchy
Call sheet layout should resemble a professional printable document:

Header
Production/day identity
Crew call
Cast calls
Location
Scenes
Practical notes
Emergency contacts
Optional weather/attachments

The application chrome surrounds the document but does not appear in the exported file.

## 91.2 Editable vs derived fields
Derived fields from the Shooting Day are presented as prefilled data. Editable fields—call times, meeting point, notes, attachments—use visible editable styling.

This makes it obvious which data came from the schedule and which is specific to the call sheet.

## 91.3 Stale banner
When source changes:

`Source Schedule Changed — Review Update`

The call sheet remains readable. Clicking Review/Refresh opens a preview.

## 91.4 Finalized visual state
A finalized call sheet shows a clear `Finalized` badge and reduced editing affordances. Opening it remains possible; creating a new snapshot/revision is the normal path for changes.

# 92. Page Action Matrix — Canonical UX

|Page|Primary action|Secondary actions|Hidden/forbidden by default|
|---|---|---|---|
|Home|Open/Create Project|Archive, Duplicate, Rename, Search|Production metadata|
|Idea Vault|Add|Search, View, Folder/Collection, Pin, Copy to Project, Export|Required classification|
|Story Board|Add Scene|Add Act/Sequence/Beat, Move, Duplicate, Park, Build Screenplay|Production fields|
|Characters|Add Character|Edit, Archive/Delete, Open Scenes, Relationships|Payroll/HR|
|Screenplay|Write/Edit|Drafts, Compare, Comment, Review, Export, Lock/Revision|Live Idea Vault sync|
|Breakdown|Tag/Suggest|Catalog, Scene filter, Complete, Reports|Silent auto-confirm|
|Production|Open selected workspace|Catalog, Locations, Cast/Crew, Visuals, Schedule, Budget, Notes/Tasks|Enterprise modules|
|Shot List|Add Shot|Reorder, Storyboard link, Export|Required camera metadata|
|Storyboard|Add Panel|Import/Draw, Reorder, Link Shot, Export|Full illustration suite|
|Shooting Schedule|Create Shooting Day|Drag scenes, Dates, Warnings, Suggestions, Export|Forced optimization|
|Call Sheets|Create Call Sheet|Edit, Refresh, Finalize, Export|Silent schedule mutation|
|Files|Add File|Move, Rename, Open, Export, Delete|Duplicate bureaucracy|
|AI|Ask|Context, Accept/Reject, Copy|Silent project mutation|
# 93. Screen State Matrix

|Workspace|Primary states|How state is shown|Allowed user behavior|
|---|---|---|---|
|Home|Empty / Recent / Archived / Project unavailable|Page content and small status message|Create/Open/Search|
|Project Home|New / Development / Writing / Production|Status + relevant shortcuts|Continue/Open|
|Idea Vault|Empty / Populated / Filtering / Preview / Unavailable|Toolbar + content state|Add/Browse/Search|
|Story Board|Empty / Populated / Filtering / Expanded / Dragging / Building / Saving / Error|Board + small status|Edit/Move/Build|
|Screenplay|Loading / Editing / Searching / Comparing / Review / Locked / Revision / Saving / Error|Draft/status + editor mode|Write/Search/Review|
|Breakdown|No Source / Loading / Reviewing / Suggesting / Suggestions Ready / Complete / Needs Review|Source banner + scene state|Tag/Suggest/Confirm|
|Schedule|No Schedule / Unscheduled Pool / Planning / Conflict Warning / Day Selected / Export Preview|Strip/day status|Drag/Date/Review|
|Call Sheet|None / Draft / Source Changed / Ready / Finalized / Superseded|Document status + source banner|Edit/Refresh/Finalize/Export|
# 94. Field-Level UX Presentation Matrix

|Object|Field|Default presentation|Advanced/optional presentation|Validation feedback|
|---|---|---|---|---|
|Project|Title|Inline in New Project dialog|Project settings if later changed|Title is required|
|Project|Type|Segmented choice/select|None|Feature/Short/Episodic|
|Project|Language|Secondary optional field|Project details|Optional free text|
|Project|Genre|Secondary optional field|Project details|Optional free text/tags|
|Project|Creator|Secondary optional field|Project details|Optional|
|Project|Status|Project Home/header selector|Status menu|Defined lifecycle labels|
|Project|Project Notes|Project Home/Notes workspace|Notes drawer|Optional|
|Idea|Type|Add menu selection|Detail view|Supported item type|
|Idea|Title|Optional detail field|Generated display name may exist|Blank valid|
|Idea|Body/Caption|Direct editor/preview|Detail view|Plain text|
|Idea|File/URL|Primary content representation|Detail metadata|Type-dependent|
|Idea|Folder|Move/organize action|Detail metadata|Optional|
|Idea|Collections|Add to collection|Detail metadata|Optional|
|Idea|Tags|Tag action|Detail metadata|Optional|
|Idea|Pin|Small toggle|None|Optional|
|Act|Title|Inline|Act detail|Required|
|Sequence|Name|Inline header|Sequence detail|Required|
|Beat|Text|Direct card edit|Expanded detail|Required for useful saved beat|
|Scene Card|Description|Card body|Expanded detail|Required for active useful scene|
|Scene Card|Heading|Expanded detail only|None|Valid screenplay heading when building|
|Character|Name|Primary|Detail|Required|
|Draft|Name|New Draft dialog|Draft detail|Required|
|Review|Source Draft|Review creation dialog|Review details|Required|
|Breakdown|Category|Category selector|None|Required|
|Breakdown|Name|Inline/tag row|Detail|Required|
|Catalog Item|Name|List/detail|Detail|Required|
|Location|Name|List/detail|Detail|Required|
|Person|Name|List/detail|Detail|Required|
|Person|Role/Department|Crew row/detail|Detail|Required for crew|
|Shot|Description|Shot card|Shot detail|Required|
|Storyboard Panel|Visual|Panel area|Panel detail|Required visual area|
|Shooting Day|Date|Day header|Day detail|Required|
|Shooting Day|Day Number|Generated/display field|Day detail|Positive/display order|
|Call Sheet|Source Day|Source header|Editor|Required|
|Task|Title|Task row|Task detail|Required|
|Budget Line|Description|Budget row|Detail|Required|
|Budget Line|Amount|Budget row|Detail|Non-negative monetary value|
# 95. Source Version and Staleness UX Matrix

|Area|Source indicator|When it changes|Primary action|
|---|---|---|---|
|Breakdown|Production Source: Draft N|Selected screenplay source changes/newer source exists|Review Production Update|
|Production Catalog context|Source indirectly through affected breakdown|Production source changes|Open affected scenes|
|Schedule|Production Source: Draft N|Schedule derived from source and source changes|Review update if offered|
|Call Sheet|Source Day + schedule status|Shooting Day changes|Refresh|
|Review Package|Based on Draft N|Current project differs|Review mapping|
|Issued Call Sheet|Finalized snapshot|Source changes later|No mutation; issue/refresh new document|
# 96. Delete, Archive and Recover UX Matrix

|Object|Primary action|Confirmation|What remains|
|---|---|---|---|
|Idea Vault item|Delete|Recoverable delete|Nothing downstream unless copied/associated|
|Act|Delete|Confirm if non-empty|Children can be moved/retained|
|Sequence|Delete|Confirm if non-empty|Scene Cards can be reassigned|
|Scene Card|Delete|Warn if linked|Linked screenplay scene remains|
|Character|Delete/Archive|Warn if linked|Screenplay text remains|
|Catalog Item|Archive/Delete|Confirm active associations|Historical references remain|
|Location|Delete/Archive|Warn schedule/history|Historical schedule remains readable|
|Shooting Day|Delete|Confirm|Call Sheet snapshots remain|
|Call Sheet|Delete|Confirm|Schedule remains|
|Draft|Delete|Confirm if current/locked|Other drafts remain|
|Project|Delete|Strong confirmation/recovery|Backup/recoverable copy as defined|
|Task|Delete|Recoverable|Related object unchanged|
# 97. Export and Snapshot UX Matrix

|Output|Primary source|UI entry|User sees before export|Project mutation|
|---|---|---|---|---|
|Screenplay PDF|Selected draft|Screenplay > Export|Draft + format + notes/revision choices|None|
|FDX|Selected draft|Screenplay > Export|Format + source|None|
|Fountain|Selected draft|Screenplay > Export|Format + source|None|
|DOCX|Selected draft|Screenplay > Export|Format + source|None|
|Outline PDF|Story Board|Story Board > Export|Included Acts/Sequences/Scenes|None|
|Moodboard PDF|Moodboard|Board > Export|Board preview|None|
|Storyboard PDF|Storyboard|Storyboard > Export|Panel preview|None|
|Shot List PDF|Shot List|Shot List > Export|Scene/shot scope|None|
|Schedule PDF|Schedule|Schedule > Export|Date/day scope|None|
|Call Sheet PDF|Call Sheet|Call Sheet > Export|Document preview|None|
|Report|Reports|Report > Export|Report type/filter|None|
|Sides|Shooting Day/scene set|Sides > Export|Selected scenes/source|None|
|Exchange package|Workspace|Share/Export Exchange|Package type/scope|None|
|Full Project Package|Project|Project > Export|Included workspaces/files|None|
# 98. Requirement Traceability — PRD to UX

|PRD ID|Priority|FSD scope|UX surface|
|---|---|---|---|
|PRD-CORE-001|P0||Application Launch/Home; Project Home; New Project Dialog|
|PRD-IDEA-001|P0|FSD-IDEA-001, FSD-IDEA-002, FSD-IDEA-014|Global Idea Vault; Project Idea Vault|
|PRD-IDEA-002|P0|FSD-IDEA-003, FSD-IDEA-004, FSD-IDEA-011, FSD-IDEA-012, FSD-IDEA-013|Idea Vault — Visual Behavior Matrix; Global/Project Idea Vault|
|PRD-IDEA-003|P0|FSD-IDEA-005, FSD-IDEA-006, FSD-IDEA-007, FSD-IDEA-008, FSD-IDEA-019|Idea Vault Views and Organization|
|PRD-IDEA-004|P0|FSD-IDEA-009, FSD-IDEA-010, FSD-IDEA-020|Idea Vault Item Detail; Global Search|
|PRD-IDEA-005|P0|FSD-IDEA-015, FSD-IDEA-016, FSD-IDEA-017|Idea Vault Move/Copy / Quick Capture|
|PRD-STORY-001|P0|FSD-STORY-001, FSD-STORY-002, FSD-STORY-003, FSD-STORY-017, FSD-STORY-018, FSD-STORY-019, FSD-STORY-028|Story Board Board/Outline; Acts; Sequences|
|PRD-STORY-002|P0|FSD-STORY-004, FSD-STORY-029, FSD-STORY-030|Beat Card|
|PRD-STORY-003|P0|FSD-STORY-005, FSD-STORY-006, FSD-STORY-007, FSD-STORY-026, FSD-STORY-027|Scene Card; Scene Card Detail|
|PRD-STORY-004|P0|FSD-STORY-008, FSD-STORY-009, FSD-STORY-010, FSD-STORY-011, FSD-STORY-012, FSD-STORY-013, FSD-STORY-014, FSD-STORY-015, FSD-STORY-016|Drag/Multi-select/Parking Lot|
|PRD-STORY-005|P0|FSD-STORY-020, FSD-STORY-021, FSD-STORY-022, FSD-STORY-023, FSD-STORY-024, FSD-STORY-025|Build Screenplay Preview|
|PRD-STORY-006|P0||Characters; Character Relationships|
|PRD-STORY-007|P1||Story Timeline|
|PRD-SCRIPT-001|P0|FSD-SCRIPT-001, FSD-SCRIPT-002, FSD-SCRIPT-003, FSD-SCRIPT-004, FSD-SCRIPT-005, FSD-SCRIPT-006, FSD-SCRIPT-007, FSD-SCRIPT-008, FSD-SCRIPT-009, FSD-SCRIPT-010, FSD-SCRIPT-011, FSD-SCRIPT-012, FSD-SCRIPT-013, FSD-SCRIPT-014, FSD-SCRIPT-015, FSD-SCRIPT-016, FSD-SCRIPT-017, FSD-SCRIPT-018, FSD-SCRIPT-019, FSD-SCRIPT-020, FSD-SCRIPT-021, FSD-SCRIPT-022|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|PRD-SCRIPT-002|P0|FSD-SCRIPT-023, FSD-SCRIPT-024, FSD-SCRIPT-025, FSD-SCRIPT-026, FSD-SCRIPT-027|Draft History; Draft Comparison|
|PRD-SCRIPT-003|P0|FSD-SCRIPT-028, FSD-SCRIPT-029, FSD-SCRIPT-030, FSD-SCRIPT-031, FSD-SCRIPT-032, FSD-SCRIPT-033|Review Round; Comments/Private Notes|
|PRD-SCRIPT-004|P1|FSD-SCRIPT-034, FSD-SCRIPT-035, FSD-SCRIPT-036, FSD-SCRIPT-037|Script Lock and Production Revision|
|PRD-SCRIPT-005|P0|FSD-SCRIPT-038, FSD-SCRIPT-039, FSD-SCRIPT-040, FSD-SCRIPT-041, FSD-SCRIPT-042, FSD-SCRIPT-043, FSD-SCRIPT-044|Screenplay Import|
|PRD-SCRIPT-006|P0|FSD-SCRIPT-045, FSD-SCRIPT-046, FSD-SCRIPT-047, FSD-SCRIPT-048, FSD-SCRIPT-049, FSD-SCRIPT-050|Screenplay Export/Print; Document Preview|
|PRD-SCRIPT-007|P2||Additional File Interchange|
|PRD-EP-001|P0||Episodic/Series|
|PRD-EP-002|P2||Deeper Episodic Continuity|
|PRD-BRK-001|P0|FSD-BREAKDOWN-001, FSD-BREAKDOWN-002, FSD-BREAKDOWN-003, FSD-BREAKDOWN-004, FSD-BREAKDOWN-005, FSD-BREAKDOWN-006, FSD-BREAKDOWN-007, FSD-BREAKDOWN-008, FSD-BREAKDOWN-009, FSD-BREAKDOWN-010, FSD-BREAKDOWN-011, FSD-BREAKDOWN-012, FSD-BREAKDOWN-013, FSD-BREAKDOWN-014, FSD-BREAKDOWN-015, FSD-BREAKDOWN-016, FSD-BREAKDOWN-017, FSD-BREAKDOWN-018, FSD-BREAKDOWN-019|Breakdown; Breakdown Suggestion Review|
|PRD-PROD-001|P0|FSD-PROD-001, FSD-PROD-002, FSD-PROD-003|Production Catalog|
|PRD-LOC-001|P0|FSD-PROD-004, FSD-PROD-005, FSD-PROD-006, FSD-PROD-007|Locations|
|PRD-CAST-001|P0|FSD-PROD-008, FSD-PROD-009, FSD-PROD-010|Cast & Crew|
|PRD-VIS-001|P0|FSD-PROD-011, FSD-PROD-012, FSD-PROD-013|Moodboards|
|PRD-STB-001|P0|FSD-PROD-014, FSD-PROD-015, FSD-PROD-016, FSD-PROD-017|Storyboards|
|PRD-STB-002|P1||Storyboards|
|PRD-STB-003|P2||Storyboards|
|PRD-SHOT-001|P0|FSD-PROD-018, FSD-PROD-019, FSD-PROD-020, FSD-PROD-021|Shot List|
|PRD-SCHED-001|P0|FSD-SCHED-001, FSD-SCHED-002, FSD-SCHED-003, FSD-SCHED-004, FSD-SCHED-005, FSD-SCHED-006, FSD-SCHED-007, FSD-SCHED-008, FSD-SCHED-009, FSD-SCHED-014, FSD-SCHED-015, FSD-SCHED-016|Shooting Schedule; Shooting Day; Schedule Assistance|
|PRD-SCHED-002|P1|FSD-SCHED-010, FSD-SCHED-011, FSD-SCHED-012, FSD-SCHED-013|Schedule Assistance|
|PRD-SCHED-003|P2||Schedule Assistance|
|PRD-CALL-001|P0|FSD-CALL-001, FSD-CALL-002, FSD-CALL-003, FSD-CALL-004, FSD-CALL-005, FSD-CALL-006, FSD-CALL-007, FSD-CALL-008, FSD-CALL-009, FSD-CALL-010, FSD-CALL-011, FSD-CALL-012|Call Sheets; Finalize/Export|
|PRD-COL-001|P1|FSD-COL-001, FSD-COL-002, FSD-COL-003, FSD-COL-004, FSD-COL-005|Permissions|
|PRD-COL-002|P0|FSD-COL-007|Script Review Exchange|
|PRD-COL-003|P1|FSD-COL-006, FSD-COL-008, FSD-COL-009, FSD-COL-010, FSD-COL-011|Page-Specific Exchange; Exchange Preview|
|PRD-COL-004|P1|FSD-COL-012, FSD-COL-013, FSD-COL-014, FSD-COL-015, FSD-COL-016|Exchange Preview|
|PRD-COL-005|P0|FSD-COL-017, FSD-COL-018, FSD-COL-019, FSD-COL-021, FSD-COL-022|LAN Host/Participant|
|PRD-COL-006|P1|FSD-COL-020|LAN Presence/Conflict|
|PRD-OFF-001|P0|FSD-OFF-001, FSD-OFF-002, FSD-OFF-003, FSD-OFF-004, FSD-OFF-005, FSD-OFF-006, FSD-OFF-007, FSD-OFF-008, FSD-OFF-009, FSD-OFF-010|Offline/Recovery; Full Project Portability|
|PRD-AI-001|P1|FSD-AI-001..FSD-AI-027|AI Assistant; AI External Disclosure; AI Global Command|
|PRD-AI-002|P2||AI Assistant advanced area|
|PRD-PROD-002|P1|FSD-PROD-022|Daily Production View|
|PRD-PROD-003|P1|FSD-PROD-023|Reports|
|PRD-PROD-004|P1|FSD-PROD-024|Sides|
|PRD-PROD-005|P1|FSD-PROD-025, FSD-PROD-026|Lightweight Budget|
|PRD-PROD-006|P1||Project Notes; Tasks; Activity History|
|PRD-PROD-007|P2||Reports (expanded later)|
|PRD-TPL-001|P1||Template Browser|
|PRD-UX-001|P0||Beginner-to-Professional Progressive Disclosure|
|PRD-CORE-002|P0|FSD-IDEA-018|Global Command/Search; Shared Interaction Patterns; Drag/Selection|
|PRD-CORE-003|P0||Application Home/Project Home/Empty-Error States|
|PRD-CORE-004|P2||Global Search|

# 99. Requirement Traceability — FSD to UX
The FSD contains 232 canonical functional requirement rows. The table below provides a direct UX destination for each requirement. This is intentionally exhaustive so a designer or QA reviewer can locate the relevant observable interface contract.

|FSD ID|PRD ID|Priority|Functional requirement|UX surface|
|---|---|---|---|---|
|FSD-IDEA-001|PRD-IDEA-001|P0|Create item|Global Idea Vault; Project Idea Vault|
|FSD-IDEA-002|PRD-IDEA-001|P0|Global/project separation|Global Idea Vault; Project Idea Vault|
|FSD-IDEA-003|PRD-IDEA-002|P0|Any file|Idea Vault — Visual Behavior Matrix; Global/Project Idea Vault|
|FSD-IDEA-004|PRD-IDEA-002|P0|Untitled item|Idea Vault — Visual Behavior Matrix; Global/Project Idea Vault|
|FSD-IDEA-005|PRD-IDEA-003|P0|Multiple views|Idea Vault Views and Organization|
|FSD-IDEA-006|PRD-IDEA-003|P0|Folder move|Idea Vault Views and Organization|
|FSD-IDEA-007|PRD-IDEA-003|P0|Collection|Idea Vault Views and Organization|
|FSD-IDEA-008|PRD-IDEA-003|P0|Pin|Idea Vault Views and Organization|
|FSD-IDEA-009|PRD-IDEA-004|P0|Search|Idea Vault Item Detail; Global Search|
|FSD-IDEA-010|PRD-IDEA-004|P0|Preview|Idea Vault Item Detail; Global Search|
|FSD-IDEA-011|PRD-IDEA-002|P0|Text note autosave|Idea Vault — Visual Behavior Matrix; Global/Project Idea Vault|
|FSD-IDEA-012|PRD-IDEA-002|P0|URL fallback|Idea Vault — Visual Behavior Matrix; Global/Project Idea Vault|
|FSD-IDEA-013|PRD-IDEA-002|P0|Voice note|Idea Vault — Visual Behavior Matrix; Global/Project Idea Vault|
|FSD-IDEA-014|PRD-IDEA-001|P0|Project copy|Global Idea Vault; Project Idea Vault|
|FSD-IDEA-015|PRD-IDEA-005|P0|Move to story|Idea Vault Move/Copy / Quick Capture|
|FSD-IDEA-016|PRD-IDEA-005|P0|Original retention|Idea Vault Move/Copy / Quick Capture|
|FSD-IDEA-017|PRD-IDEA-005|P0|No live sync|Idea Vault Move/Copy / Quick Capture|
|FSD-IDEA-018|PRD-CORE-002|P0|Soft delete|Global Command/Search; Shared Interaction Patterns; Drag/Selection|
|FSD-IDEA-019|PRD-IDEA-003|P0|Multi-select|Idea Vault Views and Organization|
|FSD-IDEA-020|PRD-IDEA-004|P0|External file awareness|Idea Vault Item Detail; Global Search|
|FSD-STORY-001|PRD-STORY-001|P0|Create Act|Story Board Board/Outline; Acts; Sequences|
|FSD-STORY-002|PRD-STORY-001|P0|Create Sequence|Story Board Board/Outline; Acts; Sequences|
|FSD-STORY-003|PRD-STORY-001|P0|Sequence contains scenes|Story Board Board/Outline; Acts; Sequences|
|FSD-STORY-004|PRD-STORY-002|P0|Create Beat|Beat Card|
|FSD-STORY-005|PRD-STORY-003|P0|Create Scene|Scene Card; Scene Card Detail|
|FSD-STORY-006|PRD-STORY-003|P0|Compact card|Scene Card; Scene Card Detail|
|FSD-STORY-007|PRD-STORY-003|P0|Expand card|Scene Card; Scene Card Detail|
|FSD-STORY-008|PRD-STORY-004|P0|Move scene|Drag/Multi-select/Parking Lot|
|FSD-STORY-009|PRD-STORY-004|P0|Move sequence|Drag/Multi-select/Parking Lot|
|FSD-STORY-010|PRD-STORY-004|P0|Move act|Drag/Multi-select/Parking Lot|
|FSD-STORY-011|PRD-STORY-004|P0|Multi-select move|Drag/Multi-select/Parking Lot|
|FSD-STORY-012|PRD-STORY-004|P0|Parking Lot|Drag/Multi-select/Parking Lot|
|FSD-STORY-013|PRD-STORY-004|P0|Duplicate card|Drag/Multi-select/Parking Lot|
|FSD-STORY-014|PRD-STORY-004|P0|No branch engine|Drag/Multi-select/Parking Lot|
|FSD-STORY-015|PRD-STORY-004|P0|Undo move|Drag/Multi-select/Parking Lot|
|FSD-STORY-016|PRD-STORY-004|P0|Undo delete|Drag/Multi-select/Parking Lot|
|FSD-STORY-017|PRD-STORY-001|P0|Outline view parity|Story Board Board/Outline; Acts; Sequences|
|FSD-STORY-018|PRD-STORY-001|P0|Collapse act|Story Board Board/Outline; Acts; Sequences|
|FSD-STORY-019|PRD-STORY-001|P0|Collapse sequence|Story Board Board/Outline; Acts; Sequences|
|FSD-STORY-020|PRD-STORY-005|P0|Build screenplay selection|Build Screenplay Preview|
|FSD-STORY-021|PRD-STORY-005|P0|Build preview|Build Screenplay Preview|
|FSD-STORY-022|PRD-STORY-005|P0|Missing heading warning|Build Screenplay Preview|
|FSD-STORY-023|PRD-STORY-005|P0|Build safety|Build Screenplay Preview|
|FSD-STORY-024|PRD-STORY-005|P0|Board independent after build|Build Screenplay Preview|
|FSD-STORY-025|PRD-STORY-005|P0|Script order warning|Build Screenplay Preview|
|FSD-STORY-026|PRD-STORY-003|P0|Characters optional|Scene Card; Scene Card Detail|
|FSD-STORY-027|PRD-STORY-003|P0|Story day optional|Scene Card; Scene Card Detail|
|FSD-STORY-028|PRD-STORY-001|P0|Sequence freeform name|Story Board Board/Outline; Acts; Sequences|
|FSD-STORY-029|PRD-STORY-002|P0|Beat conversion|Beat Card|
|FSD-STORY-030|PRD-STORY-002|P0|Scene-to-beat conversion|Beat Card|
|FSD-SCRIPT-001|PRD-SCRIPT-001|P0|New screenplay|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-002|PRD-SCRIPT-001|P0|Feature type|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-003|PRD-SCRIPT-001|P0|Short type|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-004|PRD-SCRIPT-001|P0|Episodic type|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-005|PRD-SCRIPT-001|P0|Scene heading|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-006|PRD-SCRIPT-001|P0|Action|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-007|PRD-SCRIPT-001|P0|Character|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-008|PRD-SCRIPT-001|P0|Dialogue|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-009|PRD-SCRIPT-001|P0|Parenthetical|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-010|PRD-SCRIPT-001|P0|Transition|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-011|PRD-SCRIPT-001|P0|Shot direction|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-012|PRD-SCRIPT-001|P0|Element switching|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-013|PRD-SCRIPT-001|P0|Automatic progression|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-014|PRD-SCRIPT-001|P0|Scene navigation|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-015|PRD-SCRIPT-001|P0|Search|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-016|PRD-SCRIPT-001|P0|Replace|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-017|PRD-SCRIPT-001|P0|Focus mode|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-018|PRD-SCRIPT-001|P0|Writing room|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-019|PRD-SCRIPT-001|P0|Idea Vault separation|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-020|PRD-SCRIPT-001|P0|Scene notes|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-021|PRD-SCRIPT-001|P0|Autosave|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-022|PRD-SCRIPT-001|P0|Undo/redo|Screenplay Standard; Focus; Writing Room; Navigation/Search|
|FSD-SCRIPT-023|PRD-SCRIPT-002|P0|Named draft|Draft History; Draft Comparison|
|FSD-SCRIPT-024|PRD-SCRIPT-002|P0|Draft lineage|Draft History; Draft Comparison|
|FSD-SCRIPT-025|PRD-SCRIPT-002|P0|Current draft|Draft History; Draft Comparison|
|FSD-SCRIPT-026|PRD-SCRIPT-002|P0|Automatic history|Draft History; Draft Comparison|
|FSD-SCRIPT-027|PRD-SCRIPT-002|P0|Draft comparison|Draft History; Draft Comparison|
|FSD-SCRIPT-028|PRD-SCRIPT-003|P0|Review round|Review Round; Comments/Private Notes|
|FSD-SCRIPT-029|PRD-SCRIPT-003|P0|Comment text|Review Round; Comments/Private Notes|
|FSD-SCRIPT-030|PRD-SCRIPT-003|P0|Comment scene|Review Round; Comments/Private Notes|
|FSD-SCRIPT-031|PRD-SCRIPT-003|P0|Reply|Review Round; Comments/Private Notes|
|FSD-SCRIPT-032|PRD-SCRIPT-003|P0|Resolve|Review Round; Comments/Private Notes|
|FSD-SCRIPT-033|PRD-SCRIPT-003|P0|Private note|Review Round; Comments/Private Notes|
|FSD-SCRIPT-034|PRD-SCRIPT-004|P1|Lock draft|Script Lock and Production Revision|
|FSD-SCRIPT-035|PRD-SCRIPT-004|P1|Locked edit gate|Script Lock and Production Revision|
|FSD-SCRIPT-036|PRD-SCRIPT-004|P1|Revision label|Script Lock and Production Revision|
|FSD-SCRIPT-037|PRD-SCRIPT-004|P1|Revision comparison|Script Lock and Production Revision|
|FSD-SCRIPT-038|PRD-SCRIPT-005|P0|Import FDX|Screenplay Import|
|FSD-SCRIPT-039|PRD-SCRIPT-005|P0|Import PDF|Screenplay Import|
|FSD-SCRIPT-040|PRD-SCRIPT-005|P0|Import Fountain|Screenplay Import|
|FSD-SCRIPT-041|PRD-SCRIPT-005|P0|Import TXT|Screenplay Import|
|FSD-SCRIPT-042|PRD-SCRIPT-005|P0|Import DOCX|Screenplay Import|
|FSD-SCRIPT-043|PRD-SCRIPT-005|P0|Paste screenplay|Screenplay Import|
|FSD-SCRIPT-044|PRD-SCRIPT-005|P0|Import safety|Screenplay Import|
|FSD-SCRIPT-045|PRD-SCRIPT-006|P0|Export PDF|Screenplay Export/Print; Document Preview|
|FSD-SCRIPT-046|PRD-SCRIPT-006|P0|Export FDX|Screenplay Export/Print; Document Preview|
|FSD-SCRIPT-047|PRD-SCRIPT-006|P0|Export Fountain|Screenplay Export/Print; Document Preview|
|FSD-SCRIPT-048|PRD-SCRIPT-006|P0|Export DOCX|Screenplay Export/Print; Document Preview|
|FSD-SCRIPT-049|PRD-SCRIPT-006|P0|Title page|Screenplay Export/Print; Document Preview|
|FSD-SCRIPT-050|PRD-SCRIPT-006|P0|Scene numbering|Screenplay Export/Print; Document Preview|
|FSD-BREAKDOWN-001|PRD-BRK-001|P0|Select source|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-002|PRD-BRK-001|P0|Scene list|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-003|PRD-BRK-001|P0|Script pane|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-004|PRD-BRK-001|P0|Manual tag|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-005|PRD-BRK-001|P0|Highlight tag|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-006|PRD-BRK-001|P0|Categories|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-007|PRD-BRK-001|P0|Suggest elements|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-008|PRD-BRK-001|P0|Accept suggestion|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-009|PRD-BRK-001|P0|Reject suggestion|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-010|PRD-BRK-001|P0|Edit suggestion|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-011|PRD-BRK-001|P0|Batch suggestion accept|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-012|PRD-BRK-001|P0|Catalog reuse|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-013|PRD-BRK-001|P0|New catalog item|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-014|PRD-BRK-001|P0|Remove scene association|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-015|PRD-BRK-001|P0|Breakdown complete|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-016|PRD-BRK-001|P0|Needs review|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-017|PRD-BRK-001|P0|No silent deletion|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-018|PRD-BRK-001|P0|New scene state|Breakdown; Breakdown Suggestion Review|
|FSD-BREAKDOWN-019|PRD-BRK-001|P0|Changed heading|Breakdown; Breakdown Suggestion Review|
|FSD-PROD-001|PRD-PROD-001|P0|Catalog|Production Catalog|
|FSD-PROD-002|PRD-PROD-001|P0|Catalog usage|Production Catalog|
|FSD-PROD-003|PRD-PROD-001|P0|Catalog status|Production Catalog|
|FSD-PROD-004|PRD-LOC-001|P0|Location create|Locations|
|FSD-PROD-005|PRD-LOC-001|P0|Location photos|Locations|
|FSD-PROD-006|PRD-LOC-001|P0|Location notes|Locations|
|FSD-PROD-007|PRD-LOC-001|P0|Location scenes|Locations|
|FSD-PROD-008|PRD-CAST-001|P0|Cast create|Cast & Crew|
|FSD-PROD-009|PRD-CAST-001|P0|Crew create|Cast & Crew|
|FSD-PROD-010|PRD-CAST-001|P0|Cast availability|Cast & Crew|
|FSD-PROD-011|PRD-VIS-001|P0|Moodboard create|Moodboards|
|FSD-PROD-012|PRD-VIS-001|P0|Moodboard item|Moodboards|
|FSD-PROD-013|PRD-VIS-001|P0|Moodboard export|Moodboards|
|FSD-PROD-014|PRD-STB-001|P0|Storyboard create|Storyboards|
|FSD-PROD-015|PRD-STB-001|P0|Storyboard panel|Storyboards|
|FSD-PROD-016|PRD-STB-001|P0|Storyboard reorder|Storyboards|
|FSD-PROD-017|PRD-STB-001|P0|Storyboard shot link|Storyboards|
|FSD-PROD-018|PRD-SHOT-001|P0|Shot create|Shot List|
|FSD-PROD-019|PRD-SHOT-001|P0|Shot reorder|Shot List|
|FSD-PROD-020|PRD-SHOT-001|P0|Shot optional fields|Shot List|
|FSD-PROD-021|PRD-SHOT-001|P0|Shot export|Shot List|
|FSD-PROD-022|PRD-PROD-002|P1|Daily view|Daily Production View|
|FSD-PROD-023|PRD-PROD-003|P1|Reports|Reports|
|FSD-PROD-024|PRD-PROD-004|P1|Sides|Sides|
|FSD-PROD-025|PRD-PROD-005|P1|Budget snapshot|Lightweight Budget|
|FSD-PROD-026|PRD-PROD-005|P1|Budget boundary|Lightweight Budget|
|FSD-SCHED-001|PRD-SCHED-001|P0|Create schedule|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-002|PRD-SCHED-001|P0|Unscheduled pool|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-003|PRD-SCHED-001|P0|Shooting day|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-004|PRD-SCHED-001|P0|Drag scene|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-005|PRD-SCHED-001|P0|Reorder day|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-006|PRD-SCHED-001|P0|Move day|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-007|PRD-SCHED-001|P0|Off day|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-008|PRD-SCHED-001|P0|Break marker|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-009|PRD-SCHED-001|P0|Duration|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-010|PRD-SCHED-002|P1|Conflict actor|Schedule Assistance|
|FSD-SCHED-011|PRD-SCHED-002|P1|Conflict location|Schedule Assistance|
|FSD-SCHED-012|PRD-SCHED-002|P1|Overflow|Schedule Assistance|
|FSD-SCHED-013|PRD-SCHED-002|P1|Keep anyway|Schedule Assistance|
|FSD-SCHED-014|PRD-SCHED-001|P0|Suggestions|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-015|PRD-SCHED-001|P0|Calendar|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-SCHED-016|PRD-SCHED-001|P0|Schedule export|Shooting Schedule; Shooting Day; Schedule Assistance|
|FSD-CALL-001|PRD-CALL-001|P0|Create from day|Call Sheets; Finalize/Export|
|FSD-CALL-002|PRD-CALL-001|P0|Prefill scenes|Call Sheets; Finalize/Export|
|FSD-CALL-003|PRD-CALL-001|P0|Prefill cast|Call Sheets; Finalize/Export|
|FSD-CALL-004|PRD-CALL-001|P0|Prefill location|Call Sheets; Finalize/Export|
|FSD-CALL-005|PRD-CALL-001|P0|Call time edit|Call Sheets; Finalize/Export|
|FSD-CALL-006|PRD-CALL-001|P0|Day notes|Call Sheets; Finalize/Export|
|FSD-CALL-007|PRD-CALL-001|P0|Optional sections|Call Sheets; Finalize/Export|
|FSD-CALL-008|PRD-CALL-001|P0|No reverse sync|Call Sheets; Finalize/Export|
|FSD-CALL-009|PRD-CALL-001|P0|Stale state|Call Sheets; Finalize/Export|
|FSD-CALL-010|PRD-CALL-001|P0|Refresh preview|Call Sheets; Finalize/Export|
|FSD-CALL-011|PRD-CALL-001|P0|Issued snapshot|Call Sheets; Finalize/Export|
|FSD-CALL-012|PRD-CALL-001|P0|PDF export|Call Sheets; Finalize/Export|
|FSD-COL-001|PRD-COL-001|P1|Owner|Permissions|
|FSD-COL-002|PRD-COL-001|P1|Editor|Permissions|
|FSD-COL-003|PRD-COL-001|P1|Commenter|Permissions|
|FSD-COL-004|PRD-COL-001|P1|Viewer|Permissions|
|FSD-COL-005|PRD-COL-001|P1|Export-only|Permissions|
|FSD-COL-006|PRD-COL-003|P1|Exchange story|Page-Specific Exchange; Exchange Preview|
|FSD-COL-007|PRD-COL-002|P0|Exchange script|Script Review Exchange|
|FSD-COL-008|PRD-COL-003|P1|Exchange breakdown|Page-Specific Exchange; Exchange Preview|
|FSD-COL-009|PRD-COL-003|P1|Exchange shots|Page-Specific Exchange; Exchange Preview|
|FSD-COL-010|PRD-COL-003|P1|Exchange schedule|Page-Specific Exchange; Exchange Preview|
|FSD-COL-011|PRD-COL-003|P1|Exchange call|Page-Specific Exchange; Exchange Preview|
|FSD-COL-012|PRD-COL-004|P1|Preview before apply|Exchange Preview|
|FSD-COL-013|PRD-COL-004|P1|Stale review|Exchange Preview|
|FSD-COL-014|PRD-COL-004|P1|No blind overwrite|Exchange Preview|
|FSD-COL-015|PRD-COL-004|P1|Ambiguous mapping|Exchange Preview|
|FSD-COL-016|PRD-COL-004|P1|Unmapped note|Exchange Preview|
|FSD-COL-017|PRD-COL-005|P0|Local host|LAN Host/Participant|
|FSD-COL-018|PRD-COL-005|P0|Join|LAN Host/Participant|
|FSD-COL-019|PRD-COL-005|P0|Presence|LAN Host/Participant|
|FSD-COL-020|PRD-COL-006|P1|Same-object conflict|LAN Presence/Conflict|
|FSD-COL-021|PRD-COL-005|P0|Disconnect|LAN Host/Participant|
|FSD-COL-022|PRD-COL-005|P0|End session|LAN Host/Participant|
|FSD-OFF-001|PRD-OFF-001|P0|Offline open|Offline/Recovery; Full Project Portability|
|FSD-OFF-002|PRD-OFF-001|P0|Offline edit|Offline/Recovery; Full Project Portability|
|FSD-OFF-003|PRD-OFF-001|P0|Offline production|Offline/Recovery; Full Project Portability|
|FSD-OFF-004|PRD-OFF-001|P0|Offline export|Offline/Recovery; Full Project Portability|
|FSD-OFF-005|PRD-OFF-001|P0|Save status|Offline/Recovery; Full Project Portability|
|FSD-OFF-006|PRD-OFF-001|P0|Recovery|Offline/Recovery; Full Project Portability|
|FSD-OFF-007|PRD-OFF-001|P0|Backup|Offline/Recovery; Full Project Portability|
|FSD-OFF-008|PRD-OFF-001|P0|Full project portability|Offline/Recovery; Full Project Portability|
|FSD-OFF-009|PRD-OFF-001|P0|External drive|Offline/Recovery; Full Project Portability|
|FSD-OFF-010|PRD-OFF-001|P0|Drive loss|Offline/Recovery; Full Project Portability|
|FSD-AI-001|PRD-AI-001|P1|Optional|AI Assistant; AI External Disclosure|
|FSD-AI-002|PRD-AI-001|P1|Context selector|AI Assistant; AI External Disclosure|
|FSD-AI-003|PRD-AI-001|P1|Question answering|AI Assistant; AI External Disclosure|
|FSD-AI-004|PRD-AI-001|P1|Scene card suggestion|AI Assistant; AI External Disclosure|
|FSD-AI-005|PRD-AI-001|P1|Breakdown suggestion|AI Assistant; AI External Disclosure|
|FSD-AI-006|PRD-AI-001|P1|Synopsis|AI Assistant; AI External Disclosure|
|FSD-AI-007|PRD-AI-001|P1|Schedule advice|AI Assistant; AI External Disclosure|
|FSD-AI-008|PRD-AI-001|P1|Mutation preview|AI Assistant; AI External Disclosure|
|FSD-AI-009|PRD-AI-001|P1|Accept/reject|AI Assistant; AI External Disclosure|
|FSD-AI-010|PRD-AI-001|P1|No silent rewrite|AI Assistant; AI External Disclosure|
|FSD-AI-011|PRD-AI-001|P1|External disclosure|AI Assistant; AI External Disclosure|
|FSD-AI-012|PRD-AI-001|P1|AI failure safety|AI Assistant; AI External Disclosure|
|FSD-AI-013|PRD-AI-001|P1|System-wide natural-language command interface|AI Assistant; Global Command/Search|
|FSD-AI-014|PRD-AI-001|P1|Deterministic project statistics and exact queries|AI Assistant|
|FSD-AI-015|PRD-AI-001|P1|Cross-module relationship queries|AI Assistant; Scene Hub; Production|
|FSD-AI-016|PRD-AI-001|P1|Natural-language navigation and application routing|AI Assistant; Global Command/Search|
|FSD-AI-017|PRD-AI-001|P1|Structured project-wide rename/replace|AI Assistant; Mutation Preview|
|FSD-AI-018|PRD-AI-001|P1|Batch mutation preparation and review|AI Assistant; Mutation Preview|
|FSD-AI-019|PRD-AI-001|P1|Permission inheritance|AI Assistant; Permissions|
|FSD-AI-020|PRD-AI-001|P1|Locked/private/approval state enforcement|AI Assistant; Screenplay; Review|
|FSD-AI-021|PRD-AI-001|P1|Change Set base-version validation|AI Assistant; Mutation Preview; Stale State|
|FSD-AI-022|PRD-AI-001|P1|Application tool boundary and validation|AI Assistant; Error/Recovery|
|FSD-AI-023|PRD-AI-001|P1|Product knowledge access|AI Assistant; Help/Command Access|
|FSD-AI-024|PRD-AI-001|P1|Scope/provenance disclosure|AI Assistant; AI External Disclosure|
|FSD-AI-025|PRD-AI-001|P1|AI lifecycle/activity integration|AI Assistant; Activity History; Undo/Redo|
|FSD-AI-026|PRD-AI-001|P1|Conversation/session context|AI Assistant|
|FSD-AI-027|PRD-AI-001|P1|Cross-project/episodic scope control|AI Assistant; Episodic/Season/Episode|

# 100. Cross-Module UX Event Presentation

|Event ID|Functional event|What user sees|What must not happen|
|---|---|---|---|
|EVT-001|Scene Card moved|Card visibly changes order/parent; undo available|Screenplay text does not silently change|
|EVT-002|Scene Card duplicated|New selected card appears|Original screenplay identity not copied|
|EVT-003|Scene Card parked|Card leaves active outline and appears in Parking Lot|Card is not deleted|
|EVT-004|Scene Card restored|Card appears at chosen insertion point|Identity does not change|
|EVT-005|Build Screenplay confirmed|New draft opens after progress/confirmation|Existing drafts are not overwritten|
|EVT-006|Existing script reorder request|Warning dialog appears before apply|Silent reorder|
|EVT-007|Draft created|New draft becomes selected/current per user choice|Parent draft changes|
|EVT-008|Screenplay locked|Lock badge/status appears; edits prompt revision|Locked text silently mutates|
|EVT-009|Production source updated|Source banner + impact list|Production data silently deleted|
|EVT-010|Scene removed from source|Historical/stale indicators|Schedule/breakdown silently disappear|
|EVT-011|Scene added to source|Needs Breakdown/Unscheduled state|Scene is assumed production-ready|
|EVT-012|Catalog item attached|Catalog link appears|Duplicate identity created unnecessarily|
|EVT-013|Catalog item renamed|Active references update|Historical PDFs rewrite|
|EVT-014|Schedule scene moved|Day total/call-sheet stale state updates|Screenplay reorders automatically|
|EVT-015|Schedule date changed|Affected call sheet shows stale/source-changed|Issued snapshot changes|
|EVT-016|Call sheet edited|Document changes only|Schedule changes|
|EVT-017|Call sheet refreshed|Preview shows upcoming changes|Refresh overwrites without preview|
|EVT-018|Export complete|Success + location/file action|Project content mutates|
|EVT-019|Exchange imported|Validation/change preview|Blind overwrite|
|EVT-020|AI mutation accepted|Normal undoable project action is recorded|Silent mutation|

# 101. UX Data Relationship Visualization Rules

## 101.1 One object, many focused views
The same underlying scene can be represented in Story Board, Screenplay, Breakdown, Shot List, Storyboard and Schedule. Each view should identify it using the most useful human-readable context, but the UI should not require the user to learn internal identities.

## 101.2 Scene number is display data
Scene number is generated from screenplay order. Story Board cards remain unnumbered. Production documents can display the generated number. A user moving a Story Board card does not have to renumber it.

## 101.3 Derived data labels
When a field is derived from another workspace, label it only when that prevents confusion. Examples:

`Production Source: Shooting Draft 6`

`From Shooting Day 4`

`Used in 3 scenes`

Avoid adding `Derived from database`-style technical labels.

## 101.4 Snapshot labels
Exported/issued documents should show a small state label internally such as `Finalized` or `Snapshot` where useful, but the outgoing document should remain professional rather than displaying software workflow metadata everywhere.

# 102. Navigation Consistency Matrix

|Origin object|Destination|Navigation label|Focus behavior|
|---|---|---|---|
|Scene Card|Screenplay|Convert to Screenplay Scene / Open Screenplay|Open created or linked scene|
|Scene Card|Breakdown|Open Breakdown|Open same scene in source view|
|Screenplay Scene|Story Board|Open in Story Board|Select corresponding card if present|
|Screenplay Scene|Breakdown|Break Down|Open same scene|
|Screenplay Scene|Shot List|Create Shot List|Open selected scene shot list|
|Screenplay Scene|Storyboard|Create Storyboard|Open selected scene storyboard|
|Screenplay Scene|Schedule|Open in Schedule|Show assigned/unassigned state|
|Catalog Item|Scenes|Open Scenes|Filtered scene list|
|Location|Scenes|Open Scenes|Filtered scene list|
|Person|Scenes|Open Scenes|Filtered scene list|
|Shooting Day|Call Sheet|Create Call Sheet|Open new call sheet sourced from day|
|Call Sheet|Shooting Day|Open Source Day|Open source day|
## 102.1 Navigation rule
The destination opens with the selected object already focused. The user should not land on the top of a large list and have to search again.

# 103. Information Density Rules by Screen

|Screen|Target density|Default visible information|Hidden until requested|
|---|---|---|---|
|Idea Vault|Medium/high|Content preview + minimal metadata|Optional tags/collections/source details|
|Story Board|High|Acts, sequences, short scene descriptions|Scene notes, attachments, comments|
|Scene Card detail|Medium|Heading, description, notes, attachments|Production data|
|Screenplay|Low|Script text + navigation|Secondary panels|
|Breakdown|Medium/high|Script + production categories|Advanced detail|
|Catalog|Medium|Name/category/status|Detailed notes/usage|
|Locations|Medium|Name/photo/status|Full practical notes|
|Cast/Crew|Medium|Name/role/character/contact basics|Extended notes|
|Moodboard|Visual/high|Images/notes|Metadata|
|Storyboard|Visual/medium|Panels + descriptions|Technical shot detail|
|Shot List|Medium/high|Shot order + description|Technical fields|
|Schedule|High|Days + strips + essential strip data|Detailed derived information|
|Call Sheet|Document-first|Day document|Configuration controls|
|Reports|Medium|Selected report data|Filter options|
|AI|Medium|Conversation + context|Provider settings|
## 103.1 The no-metadata-wall rule
If a user can perform the primary action without a field, the field should not occupy primary screen area by default.

# 104. Visual Semantics for Status and State

|State|Primary cue|Secondary cue|Interaction|
|---|---|---|---|
|Selected|Focus border/background|Optional check|Actions available|
|Dragging|Lifted/moved card|Insertion line|Drop to commit|
|Parked|Muted container/card treatment|Parking Lot label|Restore|
|Locked|Lock badge|Status text|Start Revision|
|Revision|Revision badge|Color on marked pages|Open/compare|
|Stale|Attention banner|Source version label|Review/Refresh|
|Suggested|Different suggestion styling|Matched text/category|Accept/Edit/Reject|
|Finalized|Stable document badge|Date/source|Open/export|
|Offline|Status strip|Local ownership message|Continue core work|
|Conflict|Attention highlight|Two-version comparison|Review/Resolve|
|Unavailable file|File-unavailable cue|Filename/path|Relink/Open location|
## 104.1 Avoid state-color ambiguity
Do not rely on a color alone. A visible label, icon, count or change in control state should communicate important status as well.

# 105. Responsive Desktop Behavior Rules

## 105.1 Wide desktop
At wide widths, Story Board can display multiple sequences side by side; Writing Room can expose one or two panels; Schedule can display several days in parallel.

## 105.2 Medium desktop
Secondary panels reduce width first. The central workspace remains readable. Story Board cards may wrap into fewer columns. Schedule can move to one day at a time while keeping the Unscheduled pool accessible.

## 105.3 Narrow desktop
Use collapsible navigation and drawers. Do not compress screenplay text until it becomes difficult to read. On narrow widths, secondary panels should close rather than force tiny writing surfaces.

## 105.4 Fullscreen
Fullscreen is especially important for Screenplay. Other workspaces can use maximized mode but should retain enough chrome to understand the current context.

# 106. Multi-Monitor Workflow Rules

## 106.1 Screenplay + Story Board
If separate windows are available, the user can place Screenplay on one monitor and Story Board on another. Each window retains project identity/context.

## 106.2 Schedule + scene context
A second window can show the selected scene's screenplay/breakdown while Stripboard remains on the primary monitor.

## 106.3 No multi-window dependency
Every workflow remains possible in one main application window. Multi-monitor is an advantage, not a requirement.

# 107. Print and External-Recipient UX

## 107.1 User mental model
The application distinguishes `working UI` from `document being sent`.

A person receiving a PDF should not see internal IDs, comments, private notes or application chrome unless the specific document format intentionally includes them.

## 107.2 Preview before send
When practical, preview should show the outgoing document close to how it will appear to the recipient.

## 107.3 Revision information
Where revision information is included, use professional labels and avoid exposing internal data model language.

## 107.4 External communication
OpenFrame creates the file; the user decides how to send it. The application does not need a mail/messaging subsystem to accomplish the workflow.

# 108. UX Rules for Project Integrity

## 108.1 Never make experimenting scary
Duplicate, park and undo should feel safe. Creative experimentation is normal and the interface should not punish it with repeated warning dialogs.

## 108.2 Warning budget
Warnings are reserved for actions that can change written screenplay order, production source state, delete connected objects, import external changes or otherwise cause meaningful loss/confusion. Routine card movement does not require confirmation.

## 108.3 No silent destructive mutation
A user should be able to explain every significant data change by looking at the visible action they took.

## 108.4 Recovery is understandable
Recovery states tell the user whether they are looking at saved content, a recovery state, an exchange preview or an issued snapshot.

# 109. Design QA Scenarios — High-Risk UX

|ID|Scenario|Action|Expected UX|
|---|---|---|---|
|HR-UX-001|Move Scene Card|Move card to another sequence|Correct order/parent; screenplay unchanged|
|HR-UX-002|Build screenplay missing heading|Build with one headerless card|Prompt to resolve/exclude|
|HR-UX-003|Reorder after script exists|Apply Story Board order|Explicit warning before screenplay order changes|
|HR-UX-004|Delete linked Scene Card|Delete card|Warn screenplay scene remains|
|HR-UX-005|Lock draft|Lock current draft|Locked state visible and safe|
|HR-UX-006|Edit locked draft|Type into locked draft|Revision prompt appears|
|HR-UX-007|Change production source|Select newer script|Impact review appears|
|HR-UX-008|Call Sheet stale|Move scheduled scene after call sheet|Call sheet becomes stale; issued snapshot remains|
|HR-UX-009|Import exchange conflict|Import older conflicting package|Preview/queue; no overwrite|
|HR-UX-010|Offline editing|Disconnect network|Core work remains available|
|HR-UX-011|LAN disconnect|Participant disconnects|Host continues; local work retained|
|HR-UX-012|AI mutation|AI proposes change|Preview + explicit accept/reject|
|HR-UX-013|Private note export|Export script with private note|Note omitted unless explicitly allowed|
|HR-UX-014|Delete Act with children|Delete non-empty Act|Move/delete/cancel choices|
|HR-UX-015|External file disappears|Open linked file after move|Unavailable state; no silent deletion|

# 110. Final Consistency Audit — PRD + FSD + UX/UI

|Audit|Coverage|Result|
|---|---|---|
|PRD requirement IDs in source|55|PASS|
|FSD functional requirement rows in source|232|PASS|
|PRD IDs represented in FSD|55 / 55|PASS|
|FSD priorities match PRD|232 / 232|PASS|
|PRD IDs mapped to UX sections|55 / 55|PASS|
|Standalone notification-center UX surface|None|PASS|
|Enterprise exclusions surfaced as UX guardrails|Yes|PASS|
|Local-first offline operation represented|Yes|PASS|
|Portable exchange collaboration represented|Yes|PASS|
|Optional LAN collaboration represented|Yes|PASS|
|AI optional with explicit mutation approval|Yes|PASS|
|Story Board not live-synced to screenplay|Yes|PASS|
|Scene Card has no permanent manual number|Yes|PASS|
|Sequences are simple scene containers|Yes|PASS|
|Production source updates are reviewable, not silent|Yes|PASS|
|Call Sheet is sourced from schedule but not reverse-synced|Yes|PASS|
## 110.1 Alignment conclusion
The PRD defines the product scope and priorities; the FSD defines the observable functional behavior; this UX/UI specification defines the corresponding screens, controls, interactions, states and visual presentation.

No UX section is intended to alter a PRD priority. No standalone feature surface has been added for excluded enterprise capabilities. The UX is intentionally detailed in interaction and presentation while preserving the core product model: local-first desktop application, simple creative development, professional screenplay writing, practical breakdown and small-team pre-production.

## 110.2 Remaining implementation discipline
During implementation, any newly proposed visible control, screen, automation or persistent workflow should be traced back to a PRD requirement and FSD requirement before being added. A visual convenience must not silently become a new product capability.

## 110.3 Release review standard
The application is ready for UX acceptance for a feature when the intended screen states, primary action, user feedback, error/recovery behavior, relationship behavior and export/visibility rules are all observable and match the corresponding FSD requirement.

# 111. Product Scope Boundary — UX View

|Area|UX status|Rule|
|---|---|---|
|Idea development|Core|Freeform, fast, unstructured|
|Story outlining|Core|Visual acts/sequences/beats/scenes; drag/drop|
|Screenwriting|Core|Professional editor, drafts, reviews|
|Breakdown|Core|Manual + confirmed suggestions|
|Visual pre-production|Core|Moodboards, basic storyboards, shot lists|
|Scheduling|Core|Stripboard + simple assistance|
|Call sheets|Core|Source from shooting day, editable snapshot|
|Budget snapshot|P1|Simple planning only|
|AI assistant|P1|Optional, scoped, user-approved mutation|
|Exchange packages|P0/P1 by type|Portable file workflow, preview before apply|
|LAN collaboration|P0/P1 by capability|Optional local session|
|Cloud sync|Excluded|No mandatory cloud|
|Accounting/payroll|Excluded|No UI surface|
|VFX/post/distribution|Excluded|No UI surface|
|Notification center|Excluded|Use contextual attention only|
|Mobile app|Excluded|Desktop-first product|
# 112. Final UX/UI Specification Statement
> **OpenFrame Studio should feel smaller than it is.**

The application contains enough workflow to take an independent filmmaker from a raw idea to a prepared shooting day, but each screen should expose only the information needed for its current job.

The intended experience is not to make filmmaking feel like enterprise administration. It is to make the professional workflow accessible:

**Capture → Arrange → Write → Review → Lock → Break Down → Visualize → Schedule → Call Sheet.**

The filmmaker owns the project, can work offline, can experiment without fear, and can exchange work as ordinary files without being forced into a cloud platform.

This UX/UI specification therefore treats simplicity as a functional requirement, not merely an aesthetic preference.
