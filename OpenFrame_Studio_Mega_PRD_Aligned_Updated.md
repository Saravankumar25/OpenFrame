# OpenFrame Studio
## Detailed Product Requirements Document — Desktop Edition

**Document status:** Product definition / implementation-grade feature specification  
**Product:** OpenFrame Studio

**Cross-cutting AI Specification:** `OpenFrame_Studio_AI_Specification_Updated.md`  
**Primary platform:** Windows + macOS desktop application  
**Core operating model:** Local-first, offline-capable, user-owned project files  
**Target users:** Independent filmmakers, writer-directors, small production teams, student filmmakers, short-film makers, regional-language filmmakers, and small episodic teams  
**Product position:** A practical, affordable, open-source-friendly alternative to StudioBinder, with an unusually good but intentionally simple idea-development and writing workflow  
**Scope focus:** Idea development, visual story outlining, screenplay writing, script review/versioning, script breakdown, visual pre-production, scheduling, and call sheets  
**Explicit scope boundary:** No attempt to become a studio ERP, accounting platform, VFX tracker, post-production asset manager, distribution CRM, payroll system, or enterprise production-management suite.

---

# 0. Document Purpose

This document replaces the earlier lightweight product blueprint with a much more explicit behavioral specification.

The purpose is to give a product designer, UX designer, product manager, QA engineer, and software engineering team a sufficiently precise description of **what OpenFrame Studio must do**, **what the user sees**, **what actions are available**, **what happens after those actions**, and **how information moves between modules**.

This is intentionally written at the **product behavior** level. It does not prescribe programming languages, databases, APIs, cloud infrastructure, UI frameworks, deployment topology, or internal code architecture.

The engineering team may choose any implementation that satisfies the behavior and acceptance criteria described here.

---

## Priority Definitions

**P0 — Product identity / first production-ready release**

A feature required for OpenFrame to deliver its core promise.

**P1 — Important second-level capability**

A feature that materially improves the product but can follow after the core workflow is stable.

**P2 — Later expansion**

A useful capability that should not block the first serious release.

**Deferred**

Explicitly outside current product scope.

> The FSD must never independently redefine feature priority. Every functional requirement inherits its priority from the PRD. If the FSD exposes a new user-visible capability not present in the PRD, the capability must first be added to the PRD or removed from the FSD.

---

# 1. Product Thesis

OpenFrame Studio should answer one practical problem:

> **“I have a movie idea, I want to develop it, write it, and prepare it for a real shoot, but professional production software is expensive, fragmented, intimidating, or unnecessarily complex for my kind of project.”**

The product therefore follows one clean journey:

```text
MESSY IDEAS
    ↓
IDEA VAULT
    ↓
VISUAL STORY BOARD
    ↓
ACTS / SEQUENCES / BEATS / SCENE CARDS
    ↓
SCREENPLAY
    ↓
REVIEW / REWRITE / VERSION
    ↓
LOCKED SHOOTING SCRIPT
    ↓
SCRIPT BREAKDOWN
    ↓
CAST / LOCATIONS / PROPS / WARDROBE / OTHER ELEMENTS
    ↓
SHOT LIST / STORYBOARD / MOODBOARD
    ↓
STRIPBOARD / SHOOTING SCHEDULE
    ↓
CALL SHEETS
```

The product should feel like **one film moving forward**, not a collection of unrelated databases.

However, this does **not** mean every module is permanently synchronized with every other module. Information should only connect where that connection creates clear practical value.

The Idea Vault is intentionally loose. The Story Board is intentionally a reference/outline space. The Screenplay is the authoritative written script. Production data becomes authoritative only when the production phase starts using it.

This distinction is critical.

---

# 2. Product Philosophy

## 2.1 Simple enough to use during real work

A filmmaker may be sitting in a bus, writing at 2 AM, discussing scenes with a friend, scouting a location, or preparing tomorrow’s shoot.

The application must remain useful in those moments.

The user should not have to stop and ask:

> “Which database field am I supposed to fill now?”

The preferred interaction is:

> Create → type → drag → continue.

---

## 2.2 Structure only when structure helps

The product must distinguish between:

**Thinking**

and

**Production administration**.

Ideas can be messy.

Story cards can be rough.

A screenplay eventually needs to be structured.

A production schedule needs much stronger structure.

Therefore the product should become progressively more structured as a project moves forward.

---

## 2.3 No duplicate entry unless the user deliberately wants a copy

The application should reuse information where it is genuinely useful.

Example:

A screenplay scene already contains:

> INT. POLICE STATION — NIGHT

When the production breakdown is created, the user should not type the location and time again.

But the Idea Vault should not constantly synchronize with the screenplay.

The user specifically chose the Vault as a creative dumping ground, so it should remain independent unless an item is explicitly moved/copied into another part of the project.

---

## 2.4 User control beats automation

Automation should reduce repetitive work, not silently make creative or production decisions.

Good automation:

- Suggest likely breakdown elements.
- Carry scene headings into production.
- Generate scene numbers from screenplay order.
- Populate a call sheet from an existing shooting day.
- Create a new script draft from a previous draft.
- Generate a synopsis from text when the user asks.

Bad automation:

- Automatically changing the story because an AI thinks the pacing is weak.
- Automatically moving scenes in the shooting schedule.
- Automatically changing the user's screenplay.
- Automatically categorizing every creative idea.
- Automatically deleting “unused” material.

---

### Cross-Cutting Product Requirements

- Local project ownership.
- Offline core functionality.
- No silent data loss.
- Undo/redo for destructive creative actions.
- Project recovery and backup.
- Exportability.
- Private-note protection.
- Clear indication when external AI services are used.
- Accessibility/basic input flexibility.
- Application responsiveness for normal supported project sizes.
- Stable behavior when external files or drives become unavailable.

---

# 3. Target User

## 3.1 Primary user

The primary user is an independent filmmaker who may be doing several jobs at once.

Examples:

- Writer-director.
- Director-producer.
- Student filmmaker.
- Independent producer.
- Assistant director on a small production.
- Short-film director.
- Regional-language filmmaker.
- Small web-series creator.
- Film-school student.
- Creator making a first feature.

The common characteristic is not job title.

It is **limited resources**.

They may not have:

- a dedicated script coordinator,
- a dedicated production manager,
- a large assistant director team,
- expensive professional software,
- multiple specialized production systems.

The application therefore needs to help one person do the work of several lightweight roles without pretending to replace an entire studio staff.

---

# 4. Users and Project Roles

The application should support these roles without building enterprise permission complexity:

| Role | Typical use |
|---|---|
| Owner | Full project control |
| Writer | Story and screenplay work |
| Director | Story, script, visual planning, shot lists |
| Producer | Project setup, production planning, schedule, documents |
| Assistant Director | Breakdown, schedule, cast/location coordination, call sheets |
| Reviewer | Comments and review |
| Contributor | Limited editing |
| Viewer | Read/export access |

A person may have multiple roles.

Example:

> One independent filmmaker can be Owner + Writer + Director + Producer.

The interface should not show different applications for each role. It should simply reveal the relevant tools and permissions.

---

# 5. Platform Definition

## 5.1 Desktop-first

OpenFrame Studio is a desktop application for:

- Windows
- macOS

The desktop application is the primary product.

A mobile application is not part of the initial product.

A browser-only replacement is not the product goal.

---

## 5.2 Local-first

The project must remain usable without an internet connection.

The user should be able to:

- create projects,
- edit ideas,
- write scripts,
- manage boards,
- build breakdowns,
- make schedules,
- create call sheets,
- export documents,

while offline.

A user should not lose access to their own work because an internet service is unavailable.

---

## 5.3 AI is optional

AI is an assistant layer, not a dependency.

All core filmmaking workflows must function without AI.

If the user chooses to use AI, the product may connect to a configured AI provider or a locally available model.

The interface must clearly indicate when content is being sent outside the local application.

AI must never be required to:

- create a script,
- create a scene card,
- break down a scene,
- schedule a film,
- export a PDF,
- generate a call sheet.

---

# 6. Product Information Model

The user should think of the product as the following hierarchy:

```text
OpenFrame Studio
│
├── Global Idea Vault
│
└── Projects
    │
    ├── Project-specific Idea Vault
    │
    ├── Project Home
    │
    ├── Story
    │   ├── Acts
    │   ├── Sequences
    │   ├── Beats
    │   ├── Scene Cards
    │   ├── Characters
    │   └── Story Timeline
    │
    ├── Screenplay
    │   ├── Current Draft
    │   ├── Draft History
    │   ├── Review Rounds
    │   └── Locked Shooting Draft
    │
    └── Production
        ├── Breakdown
        ├── Catalog
        ├── Locations
        ├── Cast & Crew
        ├── Moodboards
        ├── Storyboards
        ├── Shot Lists
        ├── Stripboard
        └── Call Sheets
```

The important design principle is that these objects are **related but not all equally coupled**.

---

# 7. Application Shell

## 7.1 Main application layout

The desktop application should use a stable shell rather than opening a new window for every tiny task.

Recommended layout:

```text
┌──────────────────────────────────────────────────────────────┐
│ OpenFrame   Project Name        Search       Help / User     │
├──────────────┬───────────────────────────────────────────────┤
│              │                                               │
│ Home         │                                               │
│ Idea Vault   │                                               │
│ Story        │              MAIN WORKSPACE                   │
│ Screenplay   │                                               │
│ Breakdown    │                                               │
│ Production   │                                               │
│ Call Sheets  │                                               │
│ Files        │                                               │
│              │                                               │
├──────────────┴───────────────────────────────────────────────┤
│ Status / autosave / current project                          │
└──────────────────────────────────────────────────────────────┘
```

The exact visual style is a design decision, but the information architecture should stay this simple.

---

## 7.2 Global navigation

Primary navigation:

1. Home
2. Idea Vault
3. Story
4. Screenplay
5. Breakdown
6. Production
7. Call Sheets
8. Files

The user should not see navigation items for:

- every category,
- every catalog type,
- every report,
- every small setting.

For example, Cast, Locations, Props and Wardrobe live inside Production rather than becoming four permanent navigation items.

---

## 7.3 Persistent project context

The application should always show:

- current project name,
- project status,
- current document/draft when relevant,
- autosave/saved status,
- current collaboration status if applicable.

The user should never wonder:

> “Which movie am I editing?”

---

# 8. Project Lifecycle

OpenFrame supports this practical lifecycle:

```text
IDEA
 ↓
DEVELOPMENT
 ↓
WRITING
 ↓
REWRITE
 ↓
SHOOTING DRAFT
 ↓
PRE-PRODUCTION
 ↓
SHOOTING PREPARATION
 ↓
CALL SHEETS
 ↓
SHOOT
```

The application should not force a project through formal gates.

A filmmaker may stay in development for months, jump back into the Idea Vault while writing, or begin breaking down a draft before it is locked.

The lifecycle is a guide, not a prison.

---

# 9. Project Creation

## 9.1 New Project action

When the user clicks **New Project**, the application should ask only:

- Project title.
- Project type.

Project type options:

- Feature Film
- Short Film
- Episodic / Series

Optional:

- Language
- Genre
- Creator

These are not required to begin writing.

---

## 9.2 New Project result

After creation, the user lands on Project Home.

Project Home should display:

> **Project title**

> “Start with your ideas, build your story, or open a screenplay.”

Primary buttons:

- Open Idea Vault
- Open Story Board
- Write Screenplay
- Import Screenplay

This ensures the user is not confronted by an empty dashboard with no obvious next step.

---

# 10. Global Idea Vault

## 10.1 Purpose

The Global Idea Vault is the filmmaker's permanent creative storage area.

It exists for things that are:

- not ready,
- not structured,
- not assigned,
- experimental,
- possibly useless,
- potentially useful years later.

The core promise is:

> **“Put anything related to a film here without first deciding what it is.”**

---

## 10.2 Supported item types

A Vault item may be:

- Text note.
- Image.
- URL.
- PDF.
- Document.
- Audio recording.
- Voice note.
- Video file.
- Folder/collection.
- Handwritten/sketch image.
- Quote.
- Screenshot.
- Any ordinary file the desktop application can safely store or reference.

The system should not require every item to have a title.

An untitled image or voice note is valid.

---

## 10.3 Global vs project Vault

There are two levels.

### Global Idea Vault

Ideas across the user's filmmaking life.

Example:

> “A thriller entirely inside a bus.”

### Project Idea Vault

Items currently associated with a particular film.

Example:

> BLACK RAIN → “Ending where evidence burns.”

The same idea may be copied into more than one project.

A copy is preferable to a live synchronization relationship because the filmmaker may evolve the idea differently in each project.

---

## 10.4 Vault views

The user can switch between:

### Visual Grid

Useful for image-heavy references.

### Card View

Useful for text notes.

### List View

Useful for large collections.

### Folder View

Useful for collections.

The user can choose their preferred view and return to it later.

---

# 11. Idea Vault Item Behavior

## 11.1 Creating a text idea

User clicks:

> + Add → Note

The application creates a note card.

The cursor immediately enters the body.

The user may type:

> “What if the hero is actually investigating his own future murder?”

The user can save it without choosing any category.

---

## 11.2 Adding an image

User chooses:

> + Add → Image

They select an image file.

The image becomes a visual card.

Optional:

- title,
- caption,
- tags,
- collection,
- source link.

None is mandatory.

---

## 11.3 Adding a URL

The user pastes a URL.

The application stores:

- URL,
- title if available,
- user-added note,
- optional screenshot/thumbnail.

The user should be able to write:

> “The architecture in this article could work for the antagonist's house.”

---

## 11.4 Voice notes

The user can click:

> Record Note

and immediately speak.

After recording, they may optionally name it.

The audio itself remains the primary source.

If speech-to-text is available, transcription is an optional convenience and must never replace the original audio.

---

## 11.5 “Anything else” behavior

Users can drag files directly into the Vault.

The application must not reject a file simply because it does not know whether it is:

> “story research” or “production reference.”

The Vault's purpose is exactly to avoid that classification requirement.

---

# 12. Idea Vault Organization

Organization is optional.

Supported organization mechanisms:

- Folders.
- Collections.
- Tags.
- Favorites/pins.
- Search.
- Recently added.
- Recently modified.

The user may create a collection such as:

> “Ending ideas.”

Another:

> “Visual references.”

Another:

> “Crazy scenes.”

The application must not prescribe folder names.

---

# 13. Moving From Idea Vault Into Story

The Vault is independent, but useful ideas may later be promoted.

For example:

> “Train station confrontation”

The user can right-click:

> **Send to Story Board**

The application asks:

> What should this become?

Options:

- Beat
- Scene Card
- Sequence idea
- Character note
- Story note

The default conversion should create a **copy**.

The original Vault item remains.

A small reference can appear:

> “Used in Story Board.”

The Vault does not then synchronize with the Story Board.

Changes made later to the scene card do not rewrite the original Vault thought.

This protects the Vault's role as creative memory.

---

# 14. Project Idea Vault

Every project automatically receives an Idea Vault.

It works exactly like the global Vault but defaults to the current project.

A project Vault is useful for:

- ending possibilities,
- alternate scenes,
- visual inspiration,
- character ideas,
- location references,
- dialogue fragments,
- research,
- music ideas,
- costume ideas,
- screenshots,
- random observations.

A user can copy an item back to the global Vault when they decide it is worth preserving outside the current film.

---

# 15. Project Home

Project Home should be a practical landing page, not an analytics dashboard.

## 15.1 Layout

Top:

> Project title
> Project status
> Last opened

Middle:

### Continue

- Continue screenplay
- Continue Story Board
- Continue breakdown
- Continue schedule

Then:

### Recent

- recently edited scene,
- recently created card,
- recently modified draft,
- recent document.

Then:

### Quick Access

- Idea Vault
- Story Board
- Screenplay
- Breakdown
- Production
- Call Sheets

Then:

### Project Files

A small file area for miscellaneous project attachments.

---

# 16. Story Workspace

The Story workspace is the product's primary creative development surface.

It must feel like a digital wall where the filmmaker can arrange a story visually.

The user should be able to understand the overall movie without reading the screenplay.

---

# 17. Story Board Views

The Story Board supports two primary views.

## 17.1 Board View

Visual cards.

Best for:

- rearranging,
- thinking,
- discussing,
- looking at the whole film.

## 17.2 Outline View

A compact textual hierarchy.

Example:

```text
ACT 1 — THE RETURN

  Sequence — Hero Introduction
    Scene — Bus Station / Night
    Scene — House / Morning
    Scene — Police Station / Afternoon

  Sequence — First Investigation
    Beat — Body discovered
    Scene — Crime Scene / Night
    Scene — Morgue / Morning

ACT 2 — THE DISCOVERY
...
```

Changing the order in either view changes the same underlying story order.

---

# 18. Acts

An Act is a high-level container.

A user creates an Act by clicking:

> + Act

The user enters only a title.

Examples:

- Act 1 — Return
- Act 2 — Investigation
- Act 3 — Confrontation

A description is optional.

An Act can contain Sequences.

The user may also place Scene Cards directly under an Act if they do not want sequences.

This is important for short films.

A five-minute short should not be forced to have complicated structure.

---

# 19. Sequences

A Sequence is simply a **group of scenes** that belong together.

The sequence has one text field:

> Sequence name

Examples:

- Hero Introduction
- Railway Station Scenes
- Investigation
- Chase
- Family Confrontation
- Final Fight
- Aftermath

The application should not require a formal “sequence purpose.”

The user can optionally add a small note, but the name is the important part.

This reflects the user's desired practical usage.

---

# 20. Beat Cards

A Beat is an idea-sized story event.

Example:

> Hero discovers the photograph.

The Beat Card is intentionally small.

Recommended visible fields:

- Beat text.

Optional:

- color,
- note,
- reference.

The user can drag beats.

A Beat can exist in the Story Board without becoming a screenplay scene.

Later, the user can convert it into a Scene Card.

---

# 21. Scene Cards

Scene Cards are the most important Story Board object.

## 21.1 Visual appearance

Scene Cards should be compact rectangular cards.
The scene header line should be inside detailed view and minimised rectangular card should only have short description
Normal card:

```text
┌─────────────────────────────┐
│ Arjun discovers the missing │
│ file...                     │
└─────────────────────────────┘
```

The card should be small enough that many scenes can fit on screen.

The description is truncated when the text is long.

There should be no giant metadata wall.

---

## 21.2 Minimum Scene Card fields

The minimum data is:

1. Short description.
2. Optional scene heading(it should be inside detailed card view but mandatory before converting to screenplay).

The scene heading is optional because the filmmaker may still be developing the scene.

No mandatory:

- scene number,
- characters,
- story day,
- production day,
- props,
- costumes,
- budget,
- shot data.

Those belong elsewhere or can be extracted later from the screenplay.

---

# 22. Scene Card Numbering

Scene Cards do **not** carry permanent scene numbers in Story Board view.

This prevents meaningless numbering work during development.

Example:

```text
Scene A
Scene B
Scene C
```

The actual screenplay scene number is generated from screenplay order when the scenes become part of a screenplay.

If the user moves a scene:

> Scene formerly numbered 23 → now appears before Scene 8.

The screenplay numbering updates accordingly.

The user does not manually renumber scenes.

---

# 23. Scene Card Expansion

Double-clicking a card opens a larger editor.

Expanded view:

```text
TITLE / HEADING

INT. POLICE STATION — NIGHT

DESCRIPTION

Arjun enters the evidence room...

NOTES

Optional freeform notes.

ATTACHMENTS

Optional images/files.
```

No unnecessary production metadata should appear here by default.

---

# 24. Drag-and-Drop Story Editing

Drag-and-drop is a core interaction, not a decoration.

The user must be able to:

- move a Scene Card within a Sequence,
- move a Scene Card to another Sequence,
- move a Scene Card directly between Acts,
- move a Sequence between Acts,
- reorder Beats,
- move Beats between Sequences,
- duplicate cards,
- send cards to Parking Lot,
- pull cards from Parking Lot.

The system should visibly show the insertion point while dragging.

After dropping:

- the story order updates immediately,
- no confirmation dialog is required,
- undo is available.

---

# 25. Parking Lot

The Story Board needs a simple side area called:

> **Parking Lot**

This is for cards that are not currently in the main story.

Examples:

- Maybe Scene 17
- Alternate ending
- Cool scene but doesn't fit
- Character idea
- Cut for now

Moving a card to Parking Lot removes it from active story order but does not delete it.

This is one of the safest ways to support experimentation without creating a complicated branching system.

---

# 26. Duplicate and Experiment

A Scene Card can be duplicated.

Example:

Original:

> Hero confronts father at house.

Duplicate:

> Hero confronts father at hospital.

Both exist independently.

The user can compare them visually and decide which survives.

The application should not attempt to automatically merge them.

---

# 27. Story Board to Screenplay

The Story Board is a reference/outline tool.

It is **not** the screenplay.

The user may:

- build an outline and then create a screenplay from it,
- start writing without using the board,
- continue editing the board after screenplay writing begins,
- use the board only as a planning reference.

There is no requirement for constant synchronization.

This is a deliberate simplification.

---

# 28. “Build Screenplay From Story Board”

When the user clicks:

> **Build Screenplay**

OpenFrame creates a screenplay structure from the active Scene Cards.

For each included card:

- screenplay scene order is created based on card position,
- the optional scene heading is carried across,
- the short description is carried into the scene as an optional planning note.

The user then writes the scene normally.

The Scene Card remains a separate reference object.

---

# 29. Story Board and Screenplay Relationship

The relationship is intentionally one-way and lightweight.

```text
STORY BOARD
    ↓
optional creation of
    ↓
SCREENPLAY SCENES
```

The Story Board can later be rearranged independently.

If a screenplay already contains written scenes, the application should warn before changing screenplay order from a Board rearrangement.

Example:

> “Reordering these cards will change the screenplay scene order. Apply change?”

Options:

- Apply
- Cancel
- Duplicate as alternate outline

This prevents accidental rewriting of a written script.

---

# 30. Characters

Characters are a lightweight story directory.

A character record contains:

- Name.
- Optional role/title.
- Optional short description.
- Optional image.
- Optional notes.
- Relationship notes.

The most important feature is not the character profile.

It is the list:

> **Scenes this character appears in.**

However, the user does not need to populate scene links manually when scenes are parsed from the screenplay.

---

# 31. Character Relationship View

A simple relationship view may show:

```text
ARJUN ───── father of ───── RAVI
  │
  └──── friend of ───────── MAYA
```

The relationship map is optional.

It must remain a visual helper rather than becoming a full character-analysis system.

---

# 32. Story Timeline

Story timeline is optional and only becomes important when time continuity matters.

Example:

```text
DAY 1
Scene 1
Scene 2
Scene 3

DAY 2
Scene 4
Scene 5
```

The user can mark the story day later.

It is not required on Scene Cards.

The screenplay and breakdown/scheduling workflows can use it when required.

---

# 33. Screenplay Workspace

The screenplay editor is the most professionally strict writing environment in the product.

It must feel like a dedicated screenplay editor rather than a word processor with a screenplay template.

---

# 34. Screenplay Element Types

The editor must support at least:

- Scene Heading.
- Action.
- Character.
- Dialogue.
- Parenthetical.
- Transition.
- Shot direction where the user chooses to use it.
- General text/notes outside the printed script.

The writer should never have to manually adjust screenplay spacing and alignment for normal writing.

---

# 35. Intelligent Element Navigation

While typing, the application should make it easy to switch between screenplay element types.

The writer should be able to:

- use keyboard shortcuts,
- use a compact element selector,
- use automatic context rules.

Example:

Typing a new scene heading and pressing Enter moves to Action.

Typing a character name and pressing Enter moves to Dialogue.

The exact keyboard shortcuts can follow industry-standard behavior where practical.

---

# 36. Screenplay Layout

The writer can use:

- Focus Mode.
- Normal editor mode.
- Editor + outline sidebar.
- Editor + scene notes panel.

The screenplay itself should remain the visual focus.

The application should not constantly cover the script with analytics.

---

# 37. Scene Navigation

A left or collapsible panel should show screenplay scenes.

Example:

```text
SCENES
01  EXT. BUS STOP — NIGHT
02  INT. APARTMENT — MORNING
03  EXT. STREET — DAY
04  INT. POLICE STATION — NIGHT
```

Clicking a scene jumps to it.

Search may find:

- dialogue,
- action,
- character names,
- scene headings.

---

# 38. Normal Find/Search

The application should not build an overpowered semantic writing search.

Normal search must support:

- text search,
- next/previous match,
- replace,
- match case,
- whole-word option.

Search works inside the screenplay currently open.

Global project search is separate.

---

# 39. Screenplay Notes

A scene can have private notes outside the printed screenplay.

Example:

> “Need stronger ending to this exchange.”

These notes do not automatically appear in exported screenplay PDFs.

A writer can toggle notes visibility.

---

# 40. Writing Room Side Panels

The user selected a simple writing room where useful reference panels can sit beside the script.

Optional panels:

- Story Board.
- Scene Notes.
- Characters.
- Comments.
- Current Draft information.

Only one or two should normally be open at once.

The user should be able to resize or close them.

The application should remember the last layout.

---

# 41. Idea Vault Is Not a Live Writing Sidebar

The Idea Vault is deliberately not embedded permanently inside the screenplay.

A writer who wants to consult it can switch to it normally.

The application should not create a complicated “linked thinking layer” between every note and every paragraph.

This follows the user's desired separation:

> Idea Vault = creative dump/reference.

> Story Board = outline/reference.

> Screenplay = actual written story.

---

# 42. Screenplay Import

The application must support importing:

- PDF.
- Final Draft `.fdx`.
- Fountain.
- TXT.
- DOCX.
- Pasted screenplay text.

Import is a first-class workflow because many filmmakers will already have scripts written elsewhere.

StudioBinder currently supports PDF, Final Draft, Fountain and TXT imports as part of its screenplay-to-production workflow. citeturn489856search14turn311070search3

OpenFrame extends this with DOCX and pasted text because the target audience includes beginners and independent filmmakers working with ordinary documents.

---

# 43. Screenplay Import Flow

User selects:

> Import Screenplay

Then selects the source file.

The application shows a preview before committing.

The preview should allow the user to confirm:

- detected scene headings,
- detected characters,
- page count,
- apparent screenplay format.

The user can then:

> Import as New Screenplay

or

> Add as New Draft.

---

# 44. Import Safety

Imported text should never replace the current screenplay automatically.

If a project already contains a screenplay, the application must create a new draft unless the user explicitly chooses to replace or import into another project.

This protects existing work.

---

# 45. Screenplay Export

Required export formats:

- PDF.
- Final Draft `.fdx`.
- Fountain.
- DOCX.

The exported screenplay should preserve:

- scene headings,
- character names,
- dialogue,
- action,
- page breaks,
- scene numbers where applicable,
- title page,
- revision information where applicable.

---

# 46. Script Drafts

The user wants both automatic history and meaningful manual drafts.

Therefore the application uses two levels.

## Automatic edit history

The application quietly preserves restore points.

The user does not need to press Save Version every ten minutes.

## Named drafts

The user explicitly creates:

- Draft 1.
- Draft 2.
- Director Rewrite.
- Producer Pass.
- Shooting Draft.

Named drafts are the drafts visible in normal version management.

---

# 47. Draft Creation

When user clicks:

> New Draft

the application asks:

**Create from:**

- Current draft.
- Previous named draft.

**Draft name:**

> Draft 4 — Director Rewrite

Optional note:

> “Reworked Act 2 and ending.”

The old draft remains unchanged.

---

# 48. Draft Comparison

The user selects:

> Compare Draft 3 → Draft 4

The comparison view shows:

- changed scenes,
- added scenes,
- removed scenes,
- changed dialogue/action,
- unchanged scenes.

The user can navigate directly to each change.

The interface should not overwhelm the user with raw machine-level diff output.

It should say:

> Scene 17 changed.

> Scene 21 removed.

> 12 lines changed in Scene 34.

Then the user can inspect the exact textual difference.

---

# 49. Review Rounds

The application supports review rounds rather than indefinite comment chaos.

Example:

> Draft 3 — Producer Review

Status:

Open notes: 12

Resolved: 8

A review round contains:

- source draft,
- reviewer(s),
- comments,
- status,
- optional deadline,
- final review state.

---

# 50. Comments

Comments can be attached to:

- screenplay text,
- scene card,
- beat card,
- sequence,
- act,
- storyboard panel,
- shot,
- location,
- breakdown item.

The same simple interaction should apply:

> Select → Comment → Write → Post.

Comments can be:

- Open.
- Resolved.

A resolved comment remains in history.

---

# 51. Private Notes

A user can create private notes that are not shared with other collaborators.

Example:

> “I don't trust this producer's suggestion; revisit later.”

Private notes should be visually distinct.

They must never accidentally export into a review package or screenplay PDF.

---

# 52. Script Lock

A script can be marked:

> Locked Shooting Draft

Locking is a formal state but should remain easy to undo for authorized users.

Before lock, the application displays a concise checklist:

- Draft name.
- Unresolved review notes.
- Current version.
- Number of scenes.
- Last modified date.

The user confirms:

> Lock this draft as the Shooting Draft.

---

# 53. Locked Script Behavior

Locking does not make the file immutable.

It changes the workflow.

When the user begins editing a locked script, the application asks:

> “This is your locked shooting draft. Create a revision?”

Options:

- Start Revision.
- Cancel.

This prevents accidental edits to the production baseline.

---

# 54. Production Revision Colors

After lock, the user can create production revisions.

The application supports revision color labels and revision pages.

The user can create:

> Revision A — Blue

or whatever revision naming sequence the production chooses.

The color itself is metadata and export styling, not a creative constraint.

The revision history shows:

- revision name,
- date,
- reason,
- changed scenes.

---

# 55. Episodic Projects

Episodic projects use:

```text
SERIES
  ↓
SEASON
  ↓
EPISODE
  ↓
STORY BOARD
  ↓
SCREENPLAY
```

Each episode can have its own:

- Idea Vault subset,
- Story Board,
- screenplay drafts,
- breakdown,
- production plan.

The series can additionally contain a lightweight shared character/location reference.

The system should not build a giant writers-room database.

---

# 56. Series-Level Story Board

At series level, the user can visually see:

```text
Episode 1
Episode 2
Episode 3
Episode 4
...
```

Optionally:

- episode title,
- one-line story summary,
- status.

This helps plan the season without forcing detailed season architecture.

---

# 57. Breakdown

Breakdown is where the screenplay begins becoming a production plan.

This workflow should resemble established production tools: select or tag screenplay elements and carry them into a catalog, schedule, and production views. StudioBinder and Celtx both emphasize this connection, and Filmustage currently uses script breakdown as the basis for scheduling and call sheets. citeturn489856search14turn489856search6turn489856search5

---

# 58. Breakdown Principle

The user should be able to select a screenplay scene and see its script.

Beside it, they see a compact breakdown tool.

Example:

```text
SCRIPT

INT. POLICE STATION — NIGHT

Arjun enters carrying a pistol.
He places the red folder on the desk.

BREAKDOWN

Cast          Arjun
Props         Pistol
Props         Red Folder
Location      Police Station
```

The user can add elements manually.

---

# 59. Automatic Breakdown Suggestions

The system may analyze the scene and suggest:

- characters,
- props,
- vehicles,
- locations,
- wardrobe,
- makeup/hair,
- effects,
- VFX,
- sound,
- animals,
- extras.

The suggestions are not final.

The interface should display:

> “Suggested elements — 6”

The user can:

- Accept.
- Reject.
- Edit.

No suggestion should silently become production data.

---

# 60. Breakdown Categories

The default categories are intentionally limited to:

1. Cast.
2. Extras / Background.
3. Location / Set.
4. Props.
5. Wardrobe.
6. Vehicles.
7. Hair / Makeup.
8. Special Effects.
9. VFX.
10. Sound.
11. Animals.

Users may add custom categories only when necessary.

---

# 61. Production Catalog

Once an element is confirmed in a breakdown, OpenFrame can add it to the project catalog.

Example:

> Red Folder

Once created, every scene using that same production item should be able to reference the same catalog entry.

The catalog is therefore a reusable directory, not one huge spreadsheet.

---

# 62. Catalog Item Details

A catalog item may contain:

- Name.
- Category.
- Description.
- Image.
- Notes.
- Scenes where it is used.
- Status.

Example:

```text
RED FOLDER
Category: Prop
Status: Required

Used in:
Scene 12
Scene 27
Scene 38
```

---

# 63. Locations

Locations should be simple production notebooks.

A Location contains:

- Name.
- Address/area.
- Contact.
- Photos.
- Notes.
- Scenes.
- Status.

Status values:

- Idea.
- Shortlisted.
- Confirmed.
- Rejected.

---

# 64. Location Notes

The location notes should support practical concerns:

- parking,
- noise,
- permission,
- access times,
- power,
- toilets,
- nearby facilities,
- travel notes,
- restrictions.

The user should be able to attach photos and reference documents.

---

# 65. Cast

Cast management remains intentionally light.

A cast record contains:

- Person name.
- Character.
- Contact information.
- Optional photo.
- Notes.
- Availability notes.
- Scenes.

No payroll.

No union accounting.

No talent contract system.

---

# 66. Crew

Crew records contain:

- Name.
- Role.
- Department.
- Contact.
- Notes.

Optional department examples:

- Direction.
- Camera.
- Sound.
- Art.
- Costume.
- Makeup.
- Production.
- Editing.

The crew module is a production directory, not HR software.

---

# 67. Moodboards

Moodboards are visual reference boards.

Examples:

- Overall Tone.
- Cinematography.
- Production Design.
- Costume.
- Lighting.
- Character.
- Locations.

Users can add:

- images,
- color references,
- notes,
- text,
- links.

They can drag items around.

---

# 68. Moodboard Export

The user can create a clean presentation/export of a moodboard.

The exported document should contain:

- project title,
- board name,
- images,
- optional captions.

Internal notes should be excluded unless explicitly selected.

---

# 69. Storyboards

Storyboarding is part of visual pre-production but must remain simpler than specialist illustration software.

Each storyboard shot is a card/panel.

Minimum information:

- Shot number.
- Image/sketch.
- Description.

Optional:

- framing,
- movement,
- dialogue,
- notes,
- duration.

The user may draw, import an image, or use a placeholder panel.

---

# 70. Script-to-Storyboard

The user may choose a scene and click:

> Create Storyboard

The application generates empty panels corresponding to shots the user wants to create.

It should not automatically invent a complete visual sequence unless the user asks an AI assistant to help.

This keeps the director in control.

StudioBinder's current storyboard workflow similarly connects screenplay scenes to visual planning and allows storyboard images to move into shot-list workflows. citeturn311070search2turn311070search7

---

# 71. Shot Lists

Shot lists are scene-based.

For each scene:

```text
SCENE 24

24A — Wide — Arjun enters station
24B — Medium — Arjun looks around
24C — Close — Red Folder
24D — OTS — Ravi
```

The user can drag shots to reorder them.

The shot list should automatically group by scene.

---

# 72. Shot Card

Required:

- Shot number.
- Description.

Optional:

- Shot size.
- Camera movement.
- Angle.
- Lens.
- Camera notes.
- Characters.
- Visual reference.
- Storyboard image.
- Sound note.

The defaults remain simple.

The user can reveal advanced fields only when needed.

---

# 73. Shot List / Storyboard Relationship

A storyboard panel may optionally be attached to a shot.

A shot may optionally contain a storyboard image.

If the user creates the storyboard first, they can use the image in the shot list.

If they create the shot first, they can later attach a storyboard panel.

Neither workflow should require the other.

---

# 74. Scheduling — Stripboard

Scheduling is a core practical feature.

The schedule should resemble the familiar stripboard workflow used by film production tools.

A strip represents one screenplay scene.

It displays:

- scene number,
- INT/EXT,
- location,
- day/night,
- page count,
- short synopsis,
- optional cast indicators,
- important breakdown indicators.

Current Filmustage documentation demonstrates both stripboard and list views, drag-and-drop reordering, day breaks, location assignment and schedule export; OpenFrame should borrow the practical model while intentionally keeping its scheduling controls lighter. citeturn489856search2turn489856search3

---

# 75. Stripboard Views

Two views:

## Board View

Colored horizontal/vertical strips for visual schedule planning.

## List View

Rows with columns for fast editing.

Both views represent the same schedule.

Changing one changes the other.

---

# 76. Schedule Creation

The user starts with:

> Create Shooting Schedule

The application gathers all screenplay scenes.

Initially all scenes are placed in an unscheduled area called:

> **Boneyard / Unscheduled**

The user drags scenes into:

> Shoot Day 1
> Shoot Day 2
> Shoot Day 3

This terminology may be adjusted in the final UI, but the concept is the same: scenes waiting to be scheduled are visually separate from scenes already scheduled.

---

# 77. Shooting Days

Each shooting day contains:

- date,
- day number,
- scenes,
- location(s),
- cast requirements,
- notes,
- estimated duration.

Optional simple markers:

- Meal Break.
- Travel.
- Company Move.
- Custom Note.

---

# 78. Schedule Assistance

The product provides suggestions, not forced optimization.

Example:

The user selects several scenes and asks:

> “Suggest a more practical shooting order.”

The assistant may point out:

> “Scenes 12, 18 and 21 all use the same location. You may reduce location changes by grouping them.”

The user can accept or ignore the suggestion.

The application must not silently rearrange the schedule.

---

# 79. Basic Schedule Conflict Detection

The application should warn about obvious conflicts.

Example:

Actor:

> Arjun

Scheduled:

Day 3 — Scene 12
Day 3 — Scene 17

Location:

Scene 12 = Location A
Scene 17 = Location B

The application says:

> **Potential conflict:** Arjun is required at two locations on the same shooting day.

It does not automatically solve it.

---

# 80. Schedule Estimation

The user may enter:

> Scene 12 — Estimated shooting time: 2 hours.

This is optional.

The day can show:

```text
Day 4

Scene 12 — 2h
Scene 15 — 1h 30m
Scene 22 — 2h

Estimated total: 5h 30m
```

The application does not need a complicated production-time formula.

---

# 81. Schedule Calendar

A simple calendar mapping should show:

```text
MON 12
Shoot Day 1

TUE 13
Shoot Day 2

WED 14
OFF

THU 15
Shoot Day 3
```

This allows the user to assign actual dates to shooting days.

The stripboard remains the main scheduling tool.

---

# 82. Call Sheets

Call sheets are generated from a scheduled shooting day.

The user selects:

> Shoot Day 4 → Create Call Sheet

OpenFrame pre-populates the document.

---

# 83. Call Sheet Required Information

The call sheet should contain a small but practical default set:

### Production

- Project title.
- Shooting date.
- Shooting day number.
- Crew call.

### Cast

- Actor.
- Character.
- Call time.

### Scenes

- Scene number.
- Scene heading.
- Short description.

### Location

- Name.
- Address.

### Practical notes

- Parking.
- Meeting point.
- Travel notes.
- Meal/break information.
- Emergency contacts.
- Production notes.

### Optional

- Weather.
- Attachments.
- Reference images.
- Special notes.

---

# 84. Call Sheet Editing

The generated call sheet remains editable.

The user can manually change:

- call times,
- notes,
- meeting points,
- attachments.

Changing the call sheet does not automatically change the main schedule unless the user explicitly chooses to update the schedule.

This prevents a document edit from unexpectedly changing production data.

---

# 85. Call Sheet Export

The default output is a clean PDF.

The document should be printable and readable on:

- desktop,
- tablet,
- phone when viewed as PDF.

Mobile viewing is supported by the exported document, not by requiring a mobile application.

---

# 86. The User's Collaboration Problem

The user does not want to maintain a cloud synchronization platform merely to enable collaboration.

OpenFrame therefore uses a **local-first collaboration model**.

This is a major product feature.

---

# 87. Collaboration Model — Three Modes

## Mode 1 — Solo Local

Default.

One person works on a local project.

No internet required.

---

## Mode 2 — Exchange Packages

For remote collaborators.

A user exports a specific page or module as an OpenFrame Exchange Package.

Examples:

> Screenplay Review Package.

> Story Board Package.

> Shot List Package.

> Breakdown Review Package.

The file can be sent by:

- email,
- messaging app,
- USB,
- file-sharing service,
- any normal file-transfer method.

The receiving person imports it into OpenFrame.

No OpenFrame cloud account is required for exchanging a package.

---

## Mode 3 — Local Network Collaboration

When several people are physically together or connected to the same private network, one machine may host a temporary collaboration session.

Other OpenFrame desktop installations connect directly to that session.

No OpenFrame cloud is required.

The host machine controls the active shared project session.

This makes simultaneous editing possible without changing the product into a cloud SaaS platform.

A private local-server model is a proven pattern in desktop creative software; DaVinci Resolve documents private Project Server and same-network collaboration patterns. citeturn311070search1turn311070search59

---

# 88. Exchange Package Principle

The package is designed to solve the user's real problem:

> “I need someone to review or edit this thing, but I don't want an online sync system just for that.”

Each exchange file should contain:

- selected content,
- required project context,
- comments if selected,
- attachments if selected,
- source version identifier,
- export timestamp.

The receiver imports the file into the corresponding OpenFrame page.

---

# 89. Review Package

Example workflow:

Writer selects Draft 4.

Clicks:

> Share → Export Review Package

Chooses:

- Script only.
- Script + comments.
- Specific scenes only.

Exports a file.

Sends it through any communication method.

Reviewer opens/imports it in OpenFrame.

Reviewer can:

- read,
- annotate,
- comment,
- mark notes.

Reviewer exports:

> Review Response Package.

Writer imports the response.

The application shows:

> 14 new comments received.

The writer chooses:

> Import Review.

Then the comments are attached to the correct source locations.

---

# 90. Merge Safety

The application must never blindly overwrite the host project during package import.

If the imported package is based on an older version, the system shows:

> “This review was created from Draft 3. Your current project is Draft 4.”

The user can:

- view the review without importing,
- import comments only,
- create a separate review record,
- compare versions.

This protects against accidental loss.

---

# 91. Page-Specific Exchange Packages

Because the user proposed a different file format for different pages, OpenFrame should formalize that idea.

Examples:

`.ofstory`

Story Board exchange.

`.ofscriptreview`

Screenplay review exchange.

`.ofbreakdown`

Breakdown exchange.

`.ofshots`

Shot-list exchange.

`.ofschedule`

Schedule exchange.

`.ofcallreview`

Call-sheet review package.

The exact file extensions may change, but the conceptual model should remain:

> **Every major OpenFrame workspace can produce a portable, self-contained exchange package.**

---

# 92. Why Exchange Packages Are Important

They provide:

- offline ownership,
- simple collaboration,
- no mandatory cloud,
- no permanent server costs,
- easy archival,
- easy external review,
- safe version boundaries.

The tradeoff is deliberate:

> Remote collaboration is a file workflow unless the users choose a direct local collaboration session.

Users can use email/WhatsApp/Drive/Dropbox/etc. themselves as the transportation layer.

OpenFrame does not need to own that communication channel.

---

# 93. AI Assistant

The AI assistant is an optional **system-wide natural-language project assistant and command interface**. It should behave more like an Amazon Q-style assistant for OpenFrame than like a screenplay judge or autonomous filmmaker.

The assistant understands two kinds of authoritative context:

- **OpenFrame knowledge:** how the application works, its terminology, permissions, source-of-truth rules, workflows and supported actions.
- **Project knowledge:** the user's accessible project data and supported relationships across the current project.

The assistant is a control layer over existing OpenFrame capabilities. It is not a second project database and never becomes authoritative over user content.

---

# 94. AI Assistant — Allowed Capabilities

The assistant may, when the user explicitly asks:

- answer questions about the current project;
- answer exact questions about structured project data;
- count characters, scenes, locations, pages, shots, production items and other supported objects;
- compare drafts and explain changes;
- locate scenes, characters, locations, files and other project references;
- traverse supported relationships across Story, Screenplay and Production;
- open a workspace, scene, draft, object or filtered result;
- summarize accessible project material;
- identify continuity concerns for human review;
- suggest Scene Cards and story structures;
- suggest breakdown elements;
- prepare synopsis or other requested document text;
- prepare schedule-grouping advice;
- prepare call-sheet drafts;
- create tasks, notes or other supported objects after confirmation;
- prepare structured project-wide rename/replace operations;
- prepare supported batch operations;
- explain stale production/schedule/call-sheet dependencies;
- explain OpenFrame workflow behavior;
- and route supported natural-language requests to the same underlying application actions available through normal UI.

The assistant should understand ordinary natural-language phrasing rather than requiring a fixed command syntax.

The assistant may also issue read-only, calculation, search and navigation commands such as:

> “Open Scene 42.”

> “Show all scenes with Meera.”

> “Find every project file mentioning railway station.”

These commands route to existing OpenFrame capabilities rather than creating a parallel application command system.

---

# 95. AI Assistant — Not Allowed by Default

The AI must not silently:

- rewrite screenplay content;
- change story canon;
- modify Scene Cards;
- rename project objects;
- replace arbitrary screenplay text;
- add breakdown elements;
- move scenes in the shooting schedule;
- modify call-sheet content;
- delete or archive project objects;
- lock or approve artifacts;
- change project permissions;
- or otherwise mutate project state.

A natural-language imperative is not blanket permission to apply a mutation.

Project-changing actions require the explicit approval flow defined in Section 97.

---

# 96. AI Scope Selector

The assistant can operate over:

- Current Selection;
- Current Scene;
- Current Screenplay;
- Specific Draft;
- Story Board;
- Selected Idea Vault items;
- Production;
- Specific Shooting Day;
- Call Sheet;
- Whole Project;
- supported explicit combinations.

The assistant should show the scope when it materially affects the answer or proposed action.

Example:

> **Using:** Draft 8 + Characters + Breakdown

The assistant may use session context for simple follow-up references such as “this scene” or “that draft.” When more than one interpretation would materially change the result or mutation, it must clarify rather than guess.

Exact counts and statistics must come from canonical project data and deterministic application calculations whenever the required structured information exists.

---

# 97. AI Action Approval

For any project-changing request:

```text
User request
↓
Resolve scope and targets
↓
Check permissions and object state
↓
Prepare exact Change Set
↓
Show preview
↓
User accepts
↓
Validate again
↓
Apply through normal OpenFrame action
↓
Undo / Activity / Recovery
```

For project-wide rename/replace, the preview must distinguish:

- canonical object names;
- structured references;
- screenplay Character elements;
- cast/Breakdown/Story Board/Shot/Storyboard references;
- optionally selected tasks/notes;
- arbitrary dialogue/action text.

Arbitrary screenplay text is not changed by default.

Batch operations must show affected counts, modules, exclusions, locked/unauthorized targets and conflicts before application.

The assistant should be able to answer supported questions about OpenFrame itself using versioned product knowledge.

AI is optional and core workflows must continue without it. A small local model, including a Qwen2.5-0.5B-class model, is a supported implementation target, but the model does not perform exact calculations, permission decisions, validation or direct project-storage writes.

AI inherits the user's effective permissions and cannot bypass private-note, locked-script, approval, lifecycle or collaboration restrictions.

AI Change Sets are bound to the project state against which they were prepared and must be revalidated if relevant project data changes.

External AI providers remain optional and require clear disclosure before project content is transmitted.

Accepted AI mutations integrate with undo/redo, activity, versioning, recovery and collaboration rules. AI failures leave project data unchanged.

---

# 98. Global Project Search

A single application search should find:

- scene cards,
- beats,
- sequences,
- characters,
- screenplay text,
- locations,
- props,
- cast,
- crew,
- files,
- Idea Vault items,
- comments.

Search results should identify the source.

Example:

```text
“red motorcycle”

SCENE 22 — Screenplay
SCENE 25 — Breakdown
PROP — Red Motorcycle
IDEA VAULT — Motorcycle reference
SHOT 25B — Shot List
```

---

# 99. Files

Projects need a simple miscellaneous file area.

Users can store:

- pitch deck,
- research PDF,
- contracts/reference documents,
- schedules received externally,
- production notes,
- location permission scans,
- posters,
- artwork.

The Files module is not a full document management system.

It is a project file cabinet.

---

# 100. Undo / Redo

Undo and redo are mandatory throughout the interactive creative workspace.

Especially for:

- drag/drop,
- deleting cards,
- moving cards,
- editing text,
- changing schedule order,
- adding/removing breakdown items.

Creative experimentation should feel safe.

---

# 101. Delete Behavior

Important objects should not disappear permanently after one accidental click.

Deletion should generally mean:

> Move to Recently Deleted.

Items may be restored.

Permanent deletion requires a deliberate second action.

---

# 102. Multi-Select

The desktop application should support multi-select wherever useful.

Examples:

- select multiple Story Cards,
- select multiple scenes,
- select multiple shots,
- select multiple Idea Vault items,
- select multiple schedule strips.

Actions may include:

- move,
- duplicate,
- export,
- delete,
- tag.

This is especially important for desktop productivity.

---

# 103. Keyboard Support

The application should be practical for people who work primarily from the keyboard.

Key actions should have shortcuts for:

- new card,
- save,
- undo,
- redo,
- search,
- next scene,
- previous scene,
- create draft,
- add comment,
- export.

The exact keybindings should be documented and customizable later.

---

# 104. Drag-and-Drop Support

Desktop drag-and-drop should work for:

- files into Idea Vault,
- files into moodboards,
- cards on Story Board,
- scenes in schedule,
- shots in shot lists,
- images into storyboards.

The dragged object should visibly show its destination.

---

# 105. Project Saving

The user should never have to think about saving every small edit.

The product should automatically preserve changes locally.

The UI should still show a small status:

> Saved.

> Saving...

> Unsaved changes.

The user should also be able to explicitly save/export a project package.

---

# 106. Project Portability

A project must be portable.

The user should be able to:

> Export Project Archive

and move the project to another machine.

The archive should preserve:

- project structure,
- Story Board,
- screenplay drafts,
- breakdown,
- catalog,
- production planning,
- comments,
- attached files where included.

The exact archive implementation is an engineering decision.

Product requirement:

> **The user owns and can move the project.**

---

# 107. External Storage

The product should allow users to choose where project files live.

Examples:

- local drive,
- external SSD,
- project folder,
- network drive where supported by the OS.

The application must not require an OpenFrame-hosted cloud folder.

---

# 108. Project Backup

The application should help the user avoid catastrophic loss.

Simple backup options:

- manual project archive,
- automatic local backup versions,
- backup location selection.

The application should explain:

> “Project backups are separate from your active project.”

---

# 109. Beginner Experience

The first-time user should be able to start from almost no film knowledge.

At project creation, offer three clear options:

> **I have an idea**

> **I already have a screenplay**

> **I am ready to plan my shoot**

### I have an idea

Opens Idea Vault + Story Board.

### I already have a screenplay

Opens import/write workflow.

### I am ready to plan my shoot

Opens screenplay import/breakdown workflow.

This avoids forcing everyone through the same path.

---

# 110. Professional Experience

Experienced filmmakers should be able to bypass guidance.

Examples:

> Import FDX → Breakdown → Stripboard.

or

> New Project → Screenplay → Shot List.

No tutorial is required to perform common actions.

---

# 111. Progressive Disclosure

The product should start simple and expose more detail when needed.

Example:

Scene Card:

```text
INT. HOUSE — NIGHT
Hero finds body.
```

Open Details:

```text
Notes
Attachments
Script connection
Comments
Production references
```

Production details appear later in Breakdown.

This prevents the development workspace from becoming a production spreadsheet.

---

# 112. Exact Connection Map

The following connections are required.

## Idea Vault → Story

Optional copy.

No continuous sync.

## Story → Screenplay

Optional generation of screenplay scene structure.

## Screenplay → Breakdown

Direct production extraction.

## Screenplay → Shot List

Scenes become shot-list containers.

## Screenplay → Storyboard

Scenes become visual planning containers.

## Breakdown → Catalog

Confirmed elements become reusable production records.

## Catalog → Schedule

Cast/location requirements can inform schedule planning.

## Schedule → Call Sheet

The shooting day populates the call sheet.

## Storyboard → Shot List

Optional visual reference transfer.

## Shot List → Call Sheet

Optional inclusion of daily shot information.

---

# 113. Connections That Are NOT Required

To keep the application simple:

Idea Vault does not automatically synchronize with screenplay.

Story Board does not automatically rewrite screenplay content.

Editing screenplay dialogue does not rewrite Story Board descriptions.

Moodboards do not automatically alter production design data.

Call sheet edits do not automatically change schedule data.

The user should understand which document is authoritative for each type of information.

---

# 114. Source of Truth Rules

| Information | Source of truth |
|---|---|
| Raw ideas | Idea Vault |
| Story order | Active Story Board or screenplay after written |
| Written dialogue/action | Screenplay draft |
| Production elements | Breakdown / Catalog |
| Shooting order | Stripboard |
| Actual call-sheet presentation | Call Sheet |
| Shot plan | Shot List |
| Visual shot reference | Storyboard |

This distinction avoids dangerous cross-module confusion.

---

# 115. Scene Identity

A screenplay scene needs a persistent internal identity even though the displayed scene number may change.

Product behavior:

- The user never sees technical identifiers unless needed.
- Scene number is generated from screenplay order.
- Moving a scene changes its displayed number.
- Breakdown information remains connected to that scene.
- Shot list items remain connected to that scene.
- Storyboard panels remain connected to that scene.
- Schedule strips refer to that scene.

This allows the production side to remain stable even while the visible scene number changes.

---

# 116. Scene Reordering Safety

If a scene has already been used in production planning and the screenplay order changes, OpenFrame should not silently destroy schedule planning.

It should display:

> “This scene is already used in production planning. Changing its order will update its script number but will not automatically reorder the shooting schedule.”

Buttons:

- Continue.
- Cancel.

This is an important distinction between **story order** and **shooting order**.

---

# 117. Breakdown After Script Change

Suppose Scene 24 originally says:

> Arjun enters carrying a phone.

The breakdown has:

> Prop: Phone.

Later the writer changes the scene to:

> Arjun enters carrying a gun.

The system should detect that the screenplay has changed but should not silently remove the phone from the production breakdown.

Instead:

> “Scene 24 changed. Review breakdown differences.”

The user can inspect:

Removed candidate:

> Phone

Added candidate:

> Gun

Buttons:

- Apply suggested update.
- Keep production data.
- Review manually.

This is safer for production.

---

# 118. Schedule After Script Change

If a locked scene changes, the schedule should remain intact until the user explicitly updates production planning.

The system may show:

> “Script changed in 3 scheduled scenes.”

Clicking opens a Review Changes panel.

No schedule change happens automatically.

---

# 119. Call Sheet After Schedule Change

If the schedule changes after a call sheet has been generated:

The application marks the call sheet:

> **Based on an older schedule version.**

The user can click:

> Update Call Sheet

and review the differences before replacing its contents.

This avoids sending an accidentally outdated call sheet.

---

# 120. Production Dashboard

The Production Dashboard is intentionally small.

It should answer:

> What do I need to do next to prepare the shoot?

Display:

- Script status.
- Breakdown progress.
- Locations still unresolved.
- Cast still unresolved.
- Schedule progress.
- Next shooting day.
- Latest call sheet.
- Important warnings.

No giant KPI wall.

---

# 121. Project Status

The user can set a simple project status:

- Idea.
- Development.
- Writing.
- Rewriting.
- Locked.
- Pre-production.
- Shooting.
- Completed.
- Archived.

The application can suggest status transitions, but the user remains in control.

---

# 122. Templates

Templates are useful for indie filmmakers because many documents repeat.

Supported templates:

- Project template.
- Call sheet template.
- Shot list template.
- Breakdown template.
- Moodboard template.
- Storyboard template.

Templates should be simple and editable.

Users should be able to save their own layout preferences.

---

# 123. Export Philosophy

Export is not secondary.

The filmmaker must be able to leave OpenFrame without being trapped.

Supported exports:

### Writing

- PDF.
- FDX.
- Fountain.
- DOCX.

### Story

- Story Board PDF.
- Outline PDF.

### Production

- Breakdown PDF.
- Catalog CSV/XLSX-style export.
- Shot list PDF.
- Storyboard PDF.
- Schedule PDF.
- Schedule spreadsheet export where practical.
- Call sheet PDF.

### Project

- Project Archive.

---

# 124. Export Selection

The user should choose:

> Export Current Page

or

> Export Selected Items

or

> Export Entire Project.

The application should never export private notes by default.

---

# 125. Printing

The documents need to be printable.

A filmmaker may print:

- shooting script,
- schedule,
- call sheet,
- shot list,
- storyboard.

The screen layout may be modern, but the exported document must remain readable when printed on standard paper sizes.

---

# 126. Empty States

Every major page needs a useful empty state.

Bad:

> No data.

Good:

### Story Board empty state

> “Start shaping the film. Add an Act, or drag an idea from your Idea Vault.”

### Screenplay empty state

> “Write from scratch or build your first screenplay scenes from the Story Board.”

### Breakdown empty state

> “Open a screenplay to begin breaking down scenes.”

### Schedule empty state

> “Break down your script first, then move scenes into shooting days.”

---

# 127. Error Philosophy

Errors should explain:

1. What happened.
2. Why it matters.
3. What the user can do.

Example:

Bad:

> Import failed.

Good:

> “The PDF could not be interpreted as a screenplay. You can still import it as a project document, or paste the screenplay text into a new draft.”

---

# 128. Safety Around Destructive Actions

Destructive actions requiring confirmation:

- Delete project.
- Delete screenplay draft.
- Permanently delete card.
- Replace screenplay with imported version.
- Overwrite an exchange package into current data.

Non-destructive actions should not require confirmation.

For example:

- Move card.
- Reorder scene.
- Add comment.
- Add shot.

---

# 129. Collaboration Permissions

Simple permissions:

### Owner

Everything.

### Editor

Edit permitted project content.

### Commenter

Read and comment.

### Viewer

Read-only.

### Export-only sharing

Receive a document/package without project editing access.

No dozens of enterprise roles.

---

# 130. Offline Collaboration State

When no network is available:

- Solo editing always works.
- Exchange packages always work.
- Local files always remain accessible.

If a collaboration session is active and the network disconnects:

- the host retains the latest local project,
- connected collaborators are informed that the shared session is unavailable,
- their local working state remains safe,
- users can later export/import exchange packages.

No data should be lost because a network disappeared.

---

# 131. Local Network Collaboration

A local collaboration session begins with:

> Start Collaboration Session

The host selects:

- project,
- allowed pages.

Other users select:

> Join Local Session

The host machine becomes the shared session authority for the current working copy.

The UI should clearly show:

> “Working in local shared session.”

---

# 132. Local Collaboration Locks

To prevent people accidentally overwriting the same text:

- selected screenplay text may display the editor currently working there,
- scene-level editing can show who is editing the scene,
- catalog edits can show the active editor.

When a user finishes, the area becomes available.

The exact locking model is technical implementation, but the product behavior should be:

> **Two people must not unknowingly overwrite each other.**

DaVinci Resolve's current collaboration model also demonstrates the usefulness of locking shared areas to avoid overwrite conflicts. citeturn311070search1

---

# 133. Remote Collaboration Without Cloud

The application should explicitly support the following workflow:

```text
Writer
  ↓
Export Review Package
  ↓
Send using normal communication method
  ↓
Reviewer
  ↓
Import Package
  ↓
Add comments
  ↓
Export Response Package
  ↓
Writer
  ↓
Import Responses
```

The platform does not attempt to replace email, WhatsApp, Telegram, Drive, Dropbox, etc.

The communication channel remains the user's choice.

---

# 134. Review Package UX

The exported review package should identify:

- Project name.
- Review target.
- Draft/version.
- Exported by.
- Date.
- Included content.

When opened, the reviewer sees a clear banner:

> “Review package — comments are separate from the original project.”

This removes ambiguity.

---

# 135. Comment Import Mapping

When review comments are imported, the system matches them against:

- screenplay draft,
- scene,
- selected text where possible.

If an exact match is no longer possible because the screenplay changed, the comment goes into:

> **Unmatched Review Notes**

The user can manually attach it to the correct scene/text.

No comment is silently discarded.

---

# 136. Story Board Exchange

A Story Board can be exported for:

- director feedback,
- writer review,
- producer discussion.

The package contains:

- acts,
- sequences,
- beats,
- scenes,
- optional comments.

The recipient can rearrange cards and export a response package.

---

# 137. Shot List Exchange

A cinematographer or collaborator can receive only the shot list package rather than the entire project.

They can:

- review shots,
- edit permitted fields,
- add comments,
- add reference images.

They return a response package.

The director imports the changes after review.

---

# 138. Breakdown Exchange

A production assistant may receive the breakdown package.

They can:

- confirm props,
- add notes,
- verify location requirements,
- add production elements.

The package can be returned without sharing the writer's private notes or entire screenplay if unnecessary.

---

# 139. Why This Matters

This collaboration model makes OpenFrame useful to small teams without forcing them into a subscription cloud platform.

It also fits the product philosophy:

> **The user owns the work.**

---

# 140. Research / Reference Boards

Research belongs primarily in Idea Vault.

For structured research, the user may create a collection:

> Research — Police Procedure

Items can include:

- PDFs,
- links,
- screenshots,
- notes,
- interviews,
- audio.

The product should not build a formal academic citation manager.

---

# 141. Project Notes

Each project may also have a simple freeform notes document.

This can be used for:

- meeting notes,
- production thoughts,
- random planning.

It does not need a formal project-management schema.

---

# 142. Lightweight Tasks

Tasks should only be included where they directly support filmmaking workflow.

Examples:

- Confirm police station.
- Rewrite Scene 22.
- Get costume reference.
- Confirm actor availability.

Tasks can be attached to:

- project,
- scene,
- location,
- breakdown item,
- review round.

Do not build a full Jira/Asana competitor.

---

# 143. Task Behavior

Each task has:

- title,
- optional assignee,
- status,
- optional due date.

Status:

- Open.
- Done.

That is enough for MVP.

---

# 144. Activity History

The application should show a lightweight project activity history.

Examples:

> Draft 4 created.

> Scene 12 moved to Sequence 3.

> Location “Old Station” confirmed.

> Shoot Day 4 created.

> Call Sheet updated.

This is primarily for orientation, not enterprise auditing.

---

# 145. No Mandatory Workflow

A filmmaker may create a screenplay without creating an Idea Vault.

They may make a Story Board without detailed character records.

They may import a finished script and immediately create a breakdown.

They may make a shot list without using storyboards.

The product should support multiple legitimate filmmaking workflows.

---

# 146. Feature Access Matrix

| Feature | Beginner | Experienced | Production team |
|---|---|---|---|
| Idea Vault | Yes | Yes | Yes |
| Story Board | Yes | Yes | Yes |
| Screenplay | Yes | Yes | Yes |
| Review | Optional | Yes | Yes |
| Breakdown | Later | Yes | Yes |
| Shot List | Optional | Yes | Yes |
| Storyboard | Optional | Yes | Yes |
| Scheduling | Later | Yes | Yes |
| Call Sheets | Later | Yes | Yes |
| AI assistant | Optional | Optional | Optional |

---

# 147. Screenplay-to-Production Handoff

When the user decides:

> “Now I want to prepare this film for shooting.”

The application should offer:

> **Start Production Setup**

The system asks which screenplay/draft should become the production source.

User chooses:

> Shooting Draft 6.

OpenFrame then creates or initializes:

- breakdown scenes,
- production catalog,
- shot-list scene containers,
- storyboard scene containers,
- scheduling source.

It must not require the user to recreate every scene manually.

---

# 148. Production Source Version

The production workspace displays:

> Production Source: Shooting Draft 6

If the user creates a new script revision, the production source remains unchanged until the user explicitly updates it.

This protects production planning.

---

# 149. Production Source Update

User clicks:

> Update Production from Draft 7

Application shows:

- scenes added,
- scenes removed,
- text changed,
- heading changed.

Then the user can:

- Update production baseline.
- Keep production on current draft.
- Review changes first.

This is a critical control point.

---

# 150. Breakdown Reconciliation

After a production source update, OpenFrame compares old and new script content.

Example:

```text
Scene 24

NEW:
Gun added.

OLD:
Phone used.
```

The application suggests:

> Add Gun to breakdown?

> Review Phone removal?

The production manager confirms.

---

# 151. Shot List Reconciliation

If Scene 24 changes but already has shots, the application keeps the existing shot list.

It flags:

> “Scene content changed. Review shots for this scene.”

This respects the filmmaker's existing work.

---

# 152. Storyboard Reconciliation

Existing storyboard panels are never silently deleted after script changes.

The application simply flags the scene:

> “Storyboard review recommended.”

---

# 153. Schedule Reconciliation

Existing schedule order remains intact after a script update.

The application flags affected scenes but does not automatically reschedule them.

This is intentionally conservative.

---

# 154. Basic Production Reports

Only practical reports should be included initially.

Examples:

- Scene breakdown.
- Cast by scene.
- Location by scene.
- Props by scene.
- Shooting schedule.
- Call sheet.

Do not build dozens of studio reports.

---

# 155. Sides

Sides are a simple useful production output.

User chooses:

> Shoot Day 4 → Generate Sides.

The document contains the screenplay pages/scenes scheduled for that day.

The user can select:

- all cast,
- selected cast,
- selected scenes.

Output is PDF.

This is a natural extension of the schedule/call-sheet workflow.

---

# 156. Daily Production View

Even though OpenFrame is pre-production focused, the user may reach the shooting stage.

A simple daily screen can show:

> Today: Shoot Day 4

Scenes:

12, 14, 18

Location:

Old Railway Station

Call Sheet:

Available

Shot List:

Available

This is enough.

Do not create a full on-set operations platform in the initial product.

---

# 157. What Happens After Shooting?

The product may allow the project to be marked:

> Shooting Complete.

But post-production is outside the core product scope.

The application should allow the filmmaker to store post-production files/notes as project files without becoming an editing or VFX system.

---

# 158. No Post-Production Expansion in Core

Explicitly excluded from the core product:

- editing timeline,
- media asset review,
- VFX tracking,
- color grading,
- sound mixing,
- dailies platform,
- delivery/QC management,
- distribution CRM.

The product ends primarily at:

> **Prepared to shoot / call-sheet-ready / small production execution.**

---

# 159. What We Are Not Building

The following are explicitly out of scope unless a future product decision changes scope:

- Studio accounting system.
- Payroll.
- Union management.
- Tax management.
- Enterprise resource planning.
- Full legal contract platform.
- Casting marketplace.
- Equipment marketplace.
- VFX tracker.
- Post-production asset manager.
- Media server.
- Dailies platform.
- Color pipeline.
- Distribution sales platform.
- Festival CRM.
- Massive rights management system.
- Enterprise SSO/identity suite.
- Complex departmental permission matrices.
- Full project management suite.
- Mandatory AI writing coach.
- Automated screenplay scoring system.

---

# 160. Feature Bloat Guardrail

A feature is rejected or postponed if it:

1. Is rarely used by independent filmmakers.
2. Requires the user to enter large amounts of metadata.
3. Exists mainly to satisfy studio-scale edge cases.
4. Duplicates a tool already used outside OpenFrame.
5. Does not connect to story or practical pre-production.
6. Adds significant interface complexity without comparable value.

---

# 161. The Four Core Jobs

OpenFrame should be excellent at exactly four jobs.

## Job 1 — Remember the film

Idea Vault.

## Job 2 — Shape the film

Story Board.

## Job 3 — Write the film

Screenplay.

## Job 4 — Prepare the film

Breakdown + pre-production + schedule + call sheet.

Everything else supports these jobs.

---

# 162. Primary User Journey — First Film

A first-time filmmaker opens OpenFrame.

### Step 1

Creates:

> My Movie

### Step 2

Clicks:

> I have an idea.

### Step 3

Idea Vault opens.

They add:

- text,
- screenshots,
- music references,
- ending idea,
- location photo.

### Step 4

They begin Story Board.

Create:

- Act 1,
- Act 2,
- Act 3.

### Step 5

Create sequences.

### Step 6

Create scene cards.

### Step 7

Drag cards until the story makes sense.

### Step 8

Build screenplay.

### Step 9

Write.

### Step 10

Invite reviewer.

### Step 11

Create Draft 2.

### Step 12

Lock shooting draft.

### Step 13

Breakdown.

### Step 14

Create locations, cast, props.

### Step 15

Create shot lists/storyboards.

### Step 16

Build stripboard.

### Step 17

Create call sheets.

The filmmaker reaches a shoot-ready state without needing separate expensive systems.

---

# 163. Primary User Journey — Existing Screenplay

Experienced filmmaker:

```text
New Project
 ↓
Import FDX
 ↓
Review scenes
 ↓
Create Breakdown
 ↓
Confirm elements
 ↓
Create Shot Lists
 ↓
Create Schedule
 ↓
Generate Call Sheets
```

They can completely bypass the Idea Vault and Story Board.

---

# 164. Primary User Journey — Short Film

A short film may contain only:

- 5 scenes,
- 1 location,
- 3 actors.

The application should not force:

- complex acts,
- detailed breakdown categories,
- elaborate schedule optimization.

A simple short-film workflow should feel fast.

---

# 165. Primary User Journey — Writer Only

A writer may use only:

- Idea Vault,
- Story Board,
- Screenplay,
- drafts,
- review.

They do not need to use Production.

---

# 166. Primary User Journey — Producer/AD

A producer can start with an imported script and use:

- Breakdown,
- Catalog,
- Cast,
- Locations,
- Schedule,
- Call Sheets.

Story tools remain available but are not required.

---

# 167. Product Behavior Around Unfinished Work

The user should be able to leave anything unfinished.

Example:

A Scene Card only contains:

> “Fight at train station.”

That's acceptable.

The application should not say:

> “You must add a scene heading.”

It can show:

> “Scene heading optional.”

Likewise, a character can exist without a profile.

A location can exist without an address.

A beat can exist without a scene.

---

# 168. Product Behavior Around Messy Creativity

The Story Board should allow rough text such as:

> “Maybe this actually happens before interval?”

The user can keep that as a card.

The application must not force clean professional terminology during development.

Professional formatting belongs in the screenplay and production stages.

---

# 169. Project Search vs Script Search

Two levels:

### Normal Script Search

Search within current screenplay.

### Global Project Search

Search across all project content.

The user should be able to choose which one they want.

---

# 170. Contextual Navigation

Every major object should have:

> **Open in...**

Example:

Scene 24:

- Open in Story Board.
- Open in Screenplay.
- Open in Breakdown.
- Open in Shot List.
- Open in Storyboard.
- Open in Schedule.

This is how the product remains connected without putting everything on one screen.

---

# 171. Scene Hub

When the user selects a screenplay scene, the application can provide a compact Scene Hub.

Example:

```text
SCENE 24
INT. POLICE STATION — NIGHT

Screenplay
Breakdown
Shots (4)
Storyboard (3)
Schedule: Shoot Day 7
Comments (2)
```

This is a high-value connection feature.

It provides a single answer to:

> “What exists around this scene?”

without merging all modules into one giant page.

---

# 172. Sequence Hub

Similarly, a Sequence can show:

- scenes,
- attached references,
- notes,
- comments.

It should not expose production details unless useful.

---

# 173. Project-level Quick Actions

A compact `+` menu should allow:

The shell may also expose an AI Assistant/Command entry. AI natural-language requests route to the same canonical application capabilities represented by Quick Actions and workspace controls; AI does not create a parallel action system.


- New Idea.
- New Act.
- New Sequence.
- New Beat.
- New Scene Card.
- New Character.
- New Location.
- New Shot List.
- New Moodboard.
- Import Screenplay.

The available options may be contextual.

---

# 174. Context Menus

Right-click behavior should be consistent.

For a scene card:

- Open.
- Duplicate.
- Move.
- Send to Parking Lot.
- Convert to Screenplay Scene.
- Add Comment.
- Export.
- Delete.

For a screenplay scene:

- Open in Story Board.
- Break down.
- Create Shot List.
- Create Storyboard.
- Add Comment.
- Copy Scene.
- Export Scene.

---

# 175. Quick Capture

Because the Idea Vault is a major differentiator, the application should make capturing thoughts extremely fast.

Global desktop action:

> Quick Capture

The user types:

> “Final scene should happen before sunrise.”

The note is immediately saved to the current project's Idea Vault.

This can be implemented as a desktop-level command later, but product behavior should support quick capture.

---

# 176. Idea Vault Favorites

A user can pin:

- core concept,
- best ending,
- main visual reference,
- important scene idea.

Pinned items appear at the top of the project Vault.

---

# 177. Idea Vault Collections

Collections are simple folders/boards.

Examples:

```text
BLACK RAIN

Ideas
Characters
Ending ideas
Visual References
Locations
Research
Music
Things to remember
```

Users decide their own structure.

---

# 178. Idea Vault Reuse

From Global Vault, user can:

> Copy to Project.

The copied item remains in the global collection.

The project copy becomes independent.

This avoids accidental changes across unrelated films.

---

# 179. Project Archive

Completed projects can be marked:

> Archived.

They remain searchable.

The user can reopen them later.

An archived project is not deleted.

---

# 180. Recent Projects

Home should show:

- recent projects,
- pinned projects,
- archived projects separately.

Each project card can show:

- title,
- type,
- status,
- last modified.

---

# 181. Performance Expectations as Product Behavior

This is not a technical architecture requirement, but the UX must remain usable with realistic indie projects.

Examples:

- 120-page screenplay.
- 150–250 scene cards across development.
- hundreds of Idea Vault items.
- several hundred reference images.
- dozens of locations.
- dozens of cast/crew records.
- hundreds of shots.

The application should not require the user to split a normal feature film into multiple projects merely to stay organized.

---

# 182. Media Handling Scope

The application can hold media references and project attachments.

It is not a media editing application.

For large media files, the product may provide normal file linking/storage choices, but it should not attempt to become a professional video editing or asset-management platform.

---

# 183. Visual Consistency

Cards should behave consistently.

Scene Card:

- compact,
- draggable,
- expandable.

Beat Card:

- compact,
- draggable.

Idea Card:

- compact,
- visual/text depending on content.

Shot Card:

- compact,
- scene-bound.

Strip:

- production-oriented.

Users should quickly learn the product's visual language.

---

# 184. Desktop Windowing

The application should support:

- resizable windows,
- maximized mode,
- fullscreen writing mode,
- optional secondary windows/panels where useful.

The core workflow should remain workable inside one main application window.

---

# 185. External Monitor Use

For desktop filmmakers, a useful future-quality feature is separate-window support.

Example:

Monitor 1:

> Screenplay

Monitor 2:

> Story Board

or

Monitor 1:

> Stripboard

Monitor 2:

> Scene script panel.

This is a desktop advantage and should be considered in design even if multi-window support is not MVP.

---

# 186. Fullscreen Writing Mode

Pressing Fullscreen Writing should hide:

- navigation,
- project dashboard,
- unnecessary panels.

Only the screenplay remains prominent.

The user should be able to return to normal mode instantly.

---

# 187. Professional Document Identity

Exports should feel professional even though the application itself is simple.

Every generated document can contain:

- project title,
- document title,
- date,
- page numbering,
- revision information where relevant.

The user can customize a small project identity block.

---

# 188. Page-Specific UX Philosophy

Each page should have a dominant question.

### Idea Vault

> “What do I have?”

### Story Board

> “How does my story fit together?”

### Screenplay

> “What is the movie on the page?”

### Breakdown

> “What does this scene require to shoot?”

### Shot List

> “How will I capture this scene?”

### Storyboard

> “What will the audience see?”

### Stripboard

> “When and where will we shoot this?”

### Call Sheet

> “What does everyone need to know for this day?”

That simplicity should guide design decisions.

---

# 189. UX Rule — Don't Show All Data Everywhere

A scene may technically have:

- screenplay text,
- characters,
- props,
- wardrobe,
- shots,
- storyboard panels,
- location,
- schedule day,
- comments.

But no single screen should show all of it by default.

Instead, provide a clear Scene Hub and dedicated views.

This is how OpenFrame stays powerful without looking complicated.

---

# 190. UX Rule — One Page, One Job

A page can contain supporting information, but it needs one clear primary job.

This is especially important for:

- Story Board,
- Screenplay,
- Breakdown,
- Stripboard.

---

# 191. UX Rule — Never Force Studio Terminology

For advanced functions, the application can show professional terms.

But whenever possible use plain language.

Examples:

> “Shooting Schedule” rather than only “Production Stripboard.”

> “Scene Cards” rather than only “Structural Units.”

> “Project Files” rather than “Asset Repository.”

This matters because the target audience includes beginners.

---

# 192. UX Rule — Let Short Films Stay Short

A user making a five-minute short should not have to complete 50 fields.

The application should let them do:

```text
Idea
↓
5 Scene Cards
↓
Screenplay
↓
Breakdown
↓
Simple Schedule
↓
Call Sheet
```

That is a complete valid OpenFrame project.

---

# 193. UX Rule — Don't Make a User Configure the Product Before Using It

There should be no large setup wizard.

The user should be able to create a project in seconds.

Configuration appears only when a specific workflow requires it.

---

# 194. UX Rule — The Product Should Teach Through Context

Instead of a long tutorial:

> “What is a Scene Card?”

The empty Story Board should simply say:

> “Scene Cards are short reminders of what happens in each scene. Drag them until the story works.”

That is enough.

---

## Requirement ID Registry

PRD requirement IDs use `PRD-[MODULE]-NNN` and identify product-level capabilities. The FSD maps its functional requirements to these parent IDs.

| PRD ID | Requirement | PRD sections | Priority |
|---|---|---|---|
| PRD-CORE-001 | Application Shell, Navigation and Project Lifecycle | 7–9, 15, 121, 179–180 | P0 |
| PRD-IDEA-001 | Global and Project Idea Vault | 10, 14 | P0 |
| PRD-IDEA-002 | Idea Vault Supported Item Types | 10.2, 11 | P0 |
| PRD-IDEA-003 | Idea Vault Views and Organization | 10.4, 12, 176–178 | P0 |
| PRD-IDEA-004 | Idea Vault Search, Preview and External File Awareness | 11, 98, 107 | P0 |
| PRD-IDEA-005 | Move/Copy Idea Vault Material into Story | 13, 112–113 | P0 |
| PRD-STORY-001 | Story Board Views, Acts and Sequences | 16–19, 17 | P0 |
| PRD-STORY-002 | Beat Cards and Beat Conversion | 20, 26–27 | P0 |
| PRD-STORY-003 | Scene Cards and Scene Card Expansion | 21–23 | P0 |
| PRD-STORY-004 | Story Board Drag-and-Drop, Parking Lot, Multi-Select and Experimentation | 24–26, 102, 104 | P0 |
| PRD-STORY-005 | Story Board to Screenplay Build | 27–29 | P0 |
| PRD-STORY-006 | Characters and Character Relationship View | 30–31 | P0 |
| PRD-STORY-007 | Story Timeline | 32 | P1 |
| PRD-SCRIPT-001 | Screenplay Workspace and Writing Controls | 33–41 | P0 |
| PRD-SCRIPT-002 | Screenplay Draft History and Comparison | 46–48 | P0 |
| PRD-SCRIPT-003 | Review Rounds, Comments and Private Notes | 49–51 | P0 |
| PRD-SCRIPT-004 | Script Lock and Production Revisions | 52–54 | P1 |
| PRD-SCRIPT-005 | Screenplay Import | 42–44 | P0 |
| PRD-SCRIPT-006 | Screenplay Export and Printing | 45, 123–125 | P0 |
| PRD-SCRIPT-007 | Additional File Interchange Formats | 197–198 | P2 |
| PRD-EP-001 | Episodic and Series Story Organization | 55–56 | P0 |
| PRD-EP-002 | Deeper Episodic Continuity | 197 | P2 |
| PRD-BRK-001 | Script Breakdown and Breakdown Suggestions | 57–60 | P0 |
| PRD-PROD-001 | Production Catalog | 61–62 | P0 |
| PRD-LOC-001 | Locations and Location Notes | 63–64 | P0 |
| PRD-CAST-001 | Cast and Crew Directory | 65–66 | P0 |
| PRD-VIS-001 | Moodboards and Visual References | 67–68 | P0 |
| PRD-STB-001 | Basic Storyboards | 69–70 | P0 |
| PRD-STB-002 | Better Storyboard Tools | 196 | P1 |
| PRD-STB-003 | Advanced Storyboard Editing | 197 | P2 |
| PRD-SHOT-001 | Shot Lists | 71–73 | P0 |
| PRD-SCHED-001 | Stripboard, Shooting Schedule and Basic Schedule Assistance | 74–81, 78 | P0 |
| PRD-SCHED-002 | Advanced Schedule Conflict Detection | 196 | P1 |
| PRD-SCHED-003 | More Sophisticated Schedule Assistance | 197 | P2 |
| PRD-CALL-001 | Call Sheets | 82–85 | P0 |
| PRD-COL-001 | Permissions | 129 | P1 |
| PRD-COL-002 | Review Packages | 89, 134 | P0 |
| PRD-COL-003 | Page-Specific Exchange Packages | 91, 136–138 | P1 |
| PRD-COL-004 | Better Exchange Package Controls | 196 | P1 |
| PRD-COL-005 | Basic Local-Network Collaboration | 86–87, 131–133 | P0 |
| PRD-COL-006 | Advanced Collaboration Conflict Handling | 131–132 | P1 |
| PRD-OFF-001 | Offline Operation, Saving, Portability, External Storage and Backup | 105–108, 130 | P0 |
| PRD-AI-001 | AI Assistant | 93–97 | P1 |
| PRD-AI-002 | More Advanced AI Project Assistance | 197 | P2 |
| PRD-PROD-002 | Daily Production View | 156 | P1 |
| PRD-PROD-003 | Basic Production Reports | 154 | P1 |
| PRD-PROD-004 | Sides | 155 | P1 |
| PRD-PROD-005 | Lightweight Budget Snapshot | 140, 154 and budgeting references | P1 |
| PRD-PROD-006 | Project Notes, Lightweight Tasks and Activity History | 141–144 | P1 |
| PRD-PROD-007 | More Production Reports | 197 | P2 |
| PRD-TPL-001 | Templates | 122 | P1 |
| PRD-UX-001 | Beginner/Professional Experience and Progressive Disclosure | 109–111, 188–194 | P0 |
| PRD-CORE-002 | Global Search, Files, Undo/Redo, Delete, Multi-Select, Keyboard and Drag-and-Drop Support | 98–104 | P0 |
| PRD-CORE-003 | Project Dashboard, Project Status, Empty States, Errors and Document Identity | 120–121, 126–127, 187 | P0 |
| PRD-CORE-004 | Richer Project Search | 197 | P2 |

## Two-Document Contract

> The PRD defines product intent, scope, priority and product-level decisions.

> The FSD defines the observable behavior required to satisfy the PRD. It may clarify behavior but may not change product intent or scope.

> When the documents conflict, the conflict must be resolved explicitly before implementation; engineering must not select whichever interpretation is more convenient.

> **FSD Scope Rule:** The FSD may decompose, clarify and add behavioral detail to PRD requirements, but it may not introduce a new user-visible product capability unless that capability is explicitly added to the PRD.

---

# 195. Feature Priority — P0

| Priority | OpenFrame features |
|---|---|
| **P0** | Desktop Windows/macOS, local projects, global/project Idea Vault, media capture, collections, search, Story Board, Acts, Sequences, Beats, Scene Cards, drag/drop, Parking Lot, screenplay editor, screenplay import/export, edit history, named drafts, draft comparison, comments, review packages, basic LAN collaboration, breakdown, breakdown suggestions, catalog, locations, cast/crew, shot lists, basic storyboards, moodboards, stripboard, basic schedule assistance, call sheets, project archive |
| **P1** | Story timeline, script lock, production revision colors, Sides, better storyboard tools, advanced schedule conflicts, AI assistant, richer exchange controls, templates, lightweight tasks, Scene Hub, more export layouts, multi-monitor, Project Notes, Activity History, Daily Production View, Basic Production Reports, Lightweight Budget |
| **P2** | More sophisticated scheduling assistance, advanced storyboard editing, richer search, more production reports, deeper episodic continuity, advanced AI assistance, additional file interchange formats |
| **Deferred** | Accounting, payroll, full budgeting, VFX management, post-production tracking, distribution, festival management, talent marketplace, cloud media hosting, mobile app, enterprise administration |

---

# 199. MVP Product Test

A user should be able to perform the following in one sitting:

```text
Create Project
↓
Write Idea
↓
Add Image Reference
↓
Create Act
↓
Create Sequence
↓
Create 10 Scene Cards
↓
Drag Scenes Around
↓
Build Screenplay
↓
Write a Scene
↓
Create Draft
↓
Break Down the Scene
↓
Create Location
↓
Create Shot
↓
Put Scene on Shoot Day 1
↓
Generate Call Sheet
↓
Export PDF
```

If any of these steps feels like switching between unrelated applications, the product is not yet coherent enough.

---

# 200. Acceptance Criteria — Idea Vault

The Idea Vault passes acceptance when:

1. A user can create a note in one interaction.
2. A user can drag an image/file into the Vault.
3. A user can create collections.
4. A user can search.
5. A user can keep an item completely unclassified.
6. A user can copy an item to a project.
7. The original global idea remains unchanged.
8. Project Vault items can exist without being used in Story Board.
9. The user can store arbitrary supported attachments.
10. Vault content is not automatically synchronized into the screenplay.

---

# 201. Acceptance Criteria — Story Board

1. Acts can be created in seconds.
2. Sequences can be created with only a title.
3. Beats can be created with only a short text.
4. Scene Cards can be created with only a description.
5. Scene heading is optional.
6. Scene Cards are compact rectangles.
7. Scene Cards can be dragged.
8. Scene Cards can move between sequences.
9. Sequences can move between acts.
10. Cards can move to Parking Lot.
11. Cards can be duplicated.
12. Undo works after movement.
13. Outline View and Board View show the same story.
14. No scene numbers need to be manually entered.

---

# 202. Acceptance Criteria — Screenplay

1. A user can create a screenplay from scratch.
2. A user can build screenplay scenes from the Story Board.
3. Scene order follows Story Board order when the screenplay is first created.
4. Scene numbers are generated from screenplay order.
5. Standard screenplay elements format automatically.
6. Search works normally.
7. Drafts can be created.
8. Previous drafts remain available.
9. Drafts can be compared.
10. Comments can be attached to text.
11. Reviews can be tracked.
12. The screenplay can be exported to PDF.
13. FDX export is available.
14. Fountain export is available.
15. DOCX export is available.
16. Import supports PDF, FDX, Fountain, TXT, DOCX and pasted text.

---

# 203. Acceptance Criteria — Breakdown

1. A screenplay scene can be opened in Breakdown.
2. User can manually tag production elements.
3. System can suggest likely production elements.
4. Suggestions require confirmation.
5. Categories remain limited by default.
6. Confirmed elements appear in Catalog.
7. Catalog items show scene usage.
8. Script changes do not silently overwrite breakdown data.
9. Changed scenes are flagged for review.

---

# 204. Acceptance Criteria — Shot List

1. User can create shots under a scene.
2. Shot cards can be reordered.
3. Basic shot information is easy to enter.
4. Technical fields remain optional.
5. Storyboard images can be attached.
6. Shot list can be exported.

---

# 205. Acceptance Criteria — Storyboard

1. User can create panels under a scene.
2. Panels can contain images or sketches.
3. Panels can be reordered.
4. Panels can contain descriptions.
5. Panels can optionally connect to shots.
6. Storyboard can be exported.

---

# 206. Acceptance Criteria — Scheduling

1. All screenplay scenes can be added to the schedule.
2. Scenes initially appear unscheduled.
3. User can create shooting days.
4. User can drag scenes into days.
5. User can reorder scenes.
6. User can assign actual dates.
7. User can see location and cast requirements.
8. System warns about obvious conflicts.
9. User can override suggestions.
10. Schedule can be exported.

---

# 207. Acceptance Criteria — Call Sheets

1. User can create a call sheet from a shooting day.
2. Schedule data populates automatically.
3. User can edit day-specific information.
4. User can add attachments.
5. Call sheet can be exported as PDF.
6. Outdated call sheets are visibly identified after relevant schedule changes.

---

# 208. Acceptance Criteria — Collaboration

1. Solo mode works offline.
2. Review packages can be exported.
3. Review packages can be imported.
4. Comments can be exchanged without cloud storage.
5. Package import never silently overwrites active work.
6. Version conflicts are shown clearly.
7. Local-network collaboration can support simultaneous users.
8. Users can fall back to package collaboration when not on the same network.

---

# 209. Acceptance Criteria — Offline

1. Project creation works offline.
2. Writing works offline.
3. Story Board works offline.
4. Breakdown works offline.
5. Scheduling works offline.
6. Call sheet generation works offline.
7. PDF exports work offline.
8. User-owned projects remain locally accessible.
9. AI is never required for core workflow.

---

# 210. Acceptance Criteria — AI

1. AI is optional.
2. User explicitly invokes AI.
3. AI shows the relevant scope when scope materially affects the result.
4. AI can answer supported project-content questions.
5. Exact project counts and statistics come from deterministic project data/calculations.
6. AI can issue supported navigation/search commands in natural language.
7. AI can traverse supported cross-module relationships.
8. AI never silently edits, deletes, archives, approves, locks or otherwise mutates project data.
9. Any project-changing AI action produces an explicit preview before mutation.
10. User can accept or reject a proposed mutation.
11. Accepted AI mutations use normal OpenFrame actions and remain undoable where the underlying action is undoable.
12. Batch mutations show affected counts and relevant exclusions/conflicts before application.
13. Structured rename/replace distinguishes canonical/structured references from arbitrary screenplay text.
14. AI respects the current user's permissions, locked states and private-note rules.
15. Stale or conflicting Change Sets are revalidated before application.
16. AI can answer supported questions about OpenFrame product behavior using current product knowledge.
17. External AI usage is clearly disclosed before project content is transmitted externally.
18. Local AI may operate without internet when configured and supported.
19. AI failure does not alter or corrupt local project data.
20. Changing the underlying model does not alter project semantics or authorization rules.

# 211. Detailed Example — Idea to Scene

User writes in Idea Vault:

> “Hero comes back to his village after ten years and finds the railway station abandoned.”

Later:

Right-click → Send to Story Board → Scene.

A Scene Card appears:

```text
Hero returns to the abandoned railway station.
```

User creates another card:

```text
He finds his childhood photograph.
```

Then another:

```text
Someone is watching him from the platform.
```

They group them under:

> Sequence — Railway Station Return

Then move that sequence into:

> Act 1

They reorder the scenes.

Then:

> Build Screenplay.

OpenFrame creates screenplay scenes in that order.

The writer writes the actual screenplay.

The Idea Vault note remains untouched.

This is the intended workflow.

---

# 212. Detailed Example — Script to Breakdown

Screenplay:

```text
INT. RAVI'S HOUSE — NIGHT

RAVI enters carrying a RED SUITCASE.
He drops a pistol on the table.
Rain hits the window.
```

User opens Breakdown.

System suggests:

```text
Cast:
Ravi

Location:
Ravi's House

Props:
Red Suitcase
Pistol

Special Effects:
Rain
```

User confirms:

- Ravi ✅
- Ravi's House ✅
- Red Suitcase ✅
- Pistol ✅
- Rain → move to Sound/Atmosphere note.

Production Catalog now contains:

> Red Suitcase

> Pistol

Those items are visible wherever the scene is used.

---

# 213. Detailed Example — Script to Shooting Plan

The screenplay has:

Scene 1 — House
Scene 2 — Street
Scene 3 — House
Scene 4 — Station
Scene 5 — House

The user knows the house can only be rented for one day.

They create:

> Shoot Day 1 — House

Drag:

Scene 1
Scene 3
Scene 5

OpenFrame shows:

> 3 scenes at same location.

The user adds:

> 8 AM call.

> 7 PM expected wrap.

Then adds Scene 2 and Scene 4 to later days.

No optimizer is required.

The filmmaker remains in control.

---

# 214. Detailed Example — Call Sheet

Shoot Day 2 contains:

Scene 2
Scene 4

Cast:

Arjun
Ravi

Location:

Railway Station

The user clicks:

> Create Call Sheet.

OpenFrame generates:

```text
BLACK RAIN
SHOOT DAY 2
12 JUNE 2027

CREW CALL: 06:00

LOCATION:
Old Railway Station
Address: ...

SCENES
2 — EXT. STREET — MORNING
4 — EXT. STATION — NIGHT

CAST
Arjun — 06:15
Ravi — 07:30

NOTES
Parking...
Meeting point...
```

The user adds weather and a special note.

Exports PDF.

Done.

---

# 215. End-to-End Data Flow

The intended practical flow is:

```text
                IDEA VAULT
                    │
                    │ optional copy
                    ▼
               STORY BOARD
          ┌─────────┼─────────┐
          ▼         ▼         ▼
         ACT     SEQUENCE    BEAT
                    │
                    ▼
                SCENE CARD
                    │
                    │ optional “Build Screenplay”
                    ▼
                SCREENPLAY
                    │
                    ▼
            LOCKED SHOOTING DRAFT
                    │
         ┌──────────┼───────────┐
         ▼          ▼           ▼
      BREAKDOWN  SHOT LIST   STORYBOARD
         │
         ▼
      CATALOG
      ┌──┼──────┬────────┐
      ▼  ▼      ▼        ▼
    CAST LOCATIONS PROPS WARDROBE
      │
      └───────────┐
                  ▼
              STRIPBOARD
                  │
                  ▼
             SHOOTING DAY
                  │
                  ▼
              CALL SHEET
```

---

# 216. Deliberate Separation of Creative and Production Data

The central design principle is:

> **Structure should increase as the film becomes real.**

Idea Vault:

Very loose.

Story Board:

Moderately structured.

Screenplay:

Highly structured.

Breakdown:

Production structured.

Schedule:

Operationally structured.

Call Sheet:

Daily actionable information.

This is the main reason the product can stay simple despite having a lot of capability.

---

# 217. Research Basis

The product deliberately references established workflows rather than inventing terminology where the film industry already has useful conventions.

## StudioBinder

Current StudioBinder materials describe screenplay importing, script breakdown, tagging production elements, script synchronization, scheduling, shot lists, storyboards, moodboards, call sheets, script versions, comments and collaboration as connected workflows. citeturn489856search14turn311070search3turn311070search5turn311070search2

OpenFrame adopts these as the practical baseline while narrowing the product around independent filmmakers and local-first use.

## Celtx

Current Celtx documentation connects beat sheets, storyboards, screenplay writing, breakdown, catalog, shot lists, schedule, cast/crew, sides and call sheets. citeturn489856search0turn489856search8

OpenFrame adopts the useful flow while deliberately resisting broad production-suite expansion.

## Filmustage

Current Filmustage documentation shows modern stripboard/list scheduling, drag-and-drop scene ordering, day breaks, estimated shooting time, scheduling views, script panels and automatic breakdown/scheduling assistance. citeturn489856search1turn489856search2turn489856search3

OpenFrame uses those practical patterns but keeps user control explicit and avoids heavy automation.

## DaVinci Resolve

DaVinci Resolve documents both cloud collaboration and private/local Project Server collaboration, including simultaneous collaboration and locking behavior. citeturn311070search0turn311070search1

This supports the feasibility of a local-network collaboration mode without making OpenFrame a cloud-first application.

---

# 218. Competitive Product Boundary

OpenFrame should not win by claiming:

> “We have more features than everybody.”

It should win by saying:

> **“The important things are all here, and they are easier to use.”**

The competitive baseline is:

### StudioBinder-style strengths

- Script.
- Breakdown.
- Shot list.
- Storyboard.
- Schedule.
- Call sheets.

### OpenFrame advantage

- Better Idea Vault.
- Better simple Story Board.
- Better connection from outline to screenplay.
- Local-first desktop ownership.
- Portable collaboration packages.
- No unnecessary studio complexity.
- Simple beginner path.
- Professional outputs.

---

# 219. Product Success Definition

The application succeeds when a user can honestly say:

> “I didn't know what I was doing when I started, but I could put my ideas somewhere, arrange my scenes, write the film, and eventually prepare a real shoot.”

And an experienced user can say:

> “I don't need all the corporate production software. I can open my screenplay, break it down, build the stripboard, and issue call sheets.”

---

# 220. Final Product Definition

> **OpenFrame Studio is a local-first desktop filmmaking workspace for independent filmmakers that turns messy movie ideas into visual story outlines, visual story outlines into professional screenplays, and finished screenplays into practical pre-production plans — without forcing indie filmmakers to use a giant studio-management system.**

---

# 221. Non-Negotiable Product Rules

1. **Idea Vault must remain messy by design.**
2. **Story Board must be visual and drag-and-drop.**
3. **Scene Cards must stay small and simple.**
4. **Scene numbers must not be manually maintained during outlining.**
5. **The Screenplay must be a serious professional writing environment.**
6. **Story Board is a reference/outline surface, not a constantly synchronized duplicate of the screenplay.**
7. **Breakdown must flow from the screenplay without re-entry.**
8. **Automatic breakdown suggestions must always require confirmation.**
9. **Scheduling must be drag-and-drop first.**
10. **Call sheets must derive from shooting days.**
11. **The core application must work offline.**
12. **Users must own their projects and be able to export them.**
13. **Remote collaboration must not require OpenFrame cloud storage.**
14. **AI must remain optional and user-controlled.**
15. **No enterprise feature may be added merely because a large studio might someday want it.**

---

# 222. Product Design Decision: The Application Must Feel Smaller Than It Is

This is a critical requirement.

OpenFrame may contain many capabilities internally, but the user should experience only a few concepts:

```text
Ideas
Story
Script
Production
```

Inside Production:

```text
Breakdown
Visuals
Schedule
Call Sheets
```

The power should come from the connections behind the interface, not from showing the user every possible object simultaneously.

---

# 223. Product Design Decision: Avoid Feature Discoverability Overload

The interface should not display:

> 80 buttons.

Instead:

- primary action,
- secondary action,
- More menu.

Contextual actions appear where relevant.

Example:

On a scene card:

> Edit
> Move
> Duplicate
> More

Under More:

> Comment
> Convert to Scene
> Export
> Delete

This keeps the workspace calm.

---

# 224. Product Design Decision: Filmmaker Ownership

The product should always make it clear:

> **This is your movie.**

OpenFrame is the workspace.

It should never behave as though the project's information belongs to the platform.

This principle affects:

- offline use,
- export,
- archives,
- collaboration packages,
- AI privacy,
- cloud requirements.

---

# 225. Product Design Decision: A User Can Ignore Half the App

A good application does not require every user to use every feature.

A writer may never use Stripboard.

A producer may never use Idea Vault.

A cinematographer may primarily use Shot List + Storyboard.

A short-film maker may use only:

> Screenplay → Breakdown → Call Sheet.

This is not a failure.

It is healthy modularity.

---

# 226. Product Design Decision: Every Feature Must Have a Clear Entry Point

For each feature, the user must know:

> Where do I find this?

Examples:

Ideas → Idea Vault.

Story → Story Board.

Writing → Screenplay.

Production requirements → Breakdown.

Visual planning → Production → Visuals.

Scheduling → Production → Schedule.

Call sheet → Call Sheets.

If a feature cannot be placed naturally, it may not belong in the core product.

---

# 227. Product Design Decision: Do Not Make the User Understand Data Models

The user does not need to know that a scene is an object shared between modules.

They simply experience:

> “This scene exists in my screenplay, my breakdown, my shot list and my schedule.”

The technical structure is the engineering team's responsibility.

The user experience must remain human.

---

# 228. Product Design Decision: Preserve Creative History

The application should make experimentation safe.

The user can:

- duplicate a Scene Card,
- move it to Parking Lot,
- create a new script draft,
- restore previous versions,
- compare drafts.

Therefore the product encourages creativity without requiring the user to manually create backup copies every time they experiment.

---

# 229. Final Scope Summary

## Core creative surface

- Global Idea Vault.
- Project Idea Vault.
- Story Board.
- Acts.
- Sequences.
- Beats.
- Scene Cards.
- Characters.
- Story Timeline.
- Screenplay.
- Drafts.
- Review/comments.

## Core pre-production surface

- Script Breakdown.
- Production Catalog.
- Cast.
- Crew.
- Locations.
- Props.
- Wardrobe.
- Moodboards.
- Storyboards.
- Shot Lists.
- Stripboard.
- Schedule.
- Call Sheets.
- Sides.

## Supporting surface

- Project Home.
- Files.
- Search.
- Tasks.
- Project Archive.
- Exports.
- Exchange Packages.
- Local Collaboration.
- Optional AI.

## Explicitly outside the core

- Post-production.
- VFX management.
- Accounting.
- Payroll.
- Distribution.
- Festival management.
- Enterprise studio management.

---

# 230. Engineering Handoff Rule

The engineering team should treat this PRD as the product behavior contract.

When implementation questions arise, use this order of priority:

1. Preserve the simple filmmaker workflow.
2. Preserve user ownership/offline operation.
3. Preserve the stated source-of-truth relationships.
4. Avoid duplicate data entry.
5. Avoid silent destructive synchronization.
6. Prefer explicit user actions over hidden automation.
7. Prefer fewer controls with sensible defaults.
8. Keep advanced behavior behind optional controls.
9. Never add enterprise complexity without a demonstrated user requirement.

If a proposed implementation makes the application technically elegant but makes the filmmaker's workflow harder, the workflow wins.

---

# 231. Final Product Mantra

```text
CAPTURE THE IDEA.

SHAPE THE STORY.

WRITE THE SCRIPT.

PLAN THE SHOOT.
```

That is OpenFrame Studio.

Not an ERP.

Not a cloud collaboration company.

Not a post-production suite.

Not an AI screenplay generator.

A focused filmmaking workspace for people who actually want to make films.

---

# Appendix A — Reference Sources Reviewed

The following public product documentation was reviewed while expanding this specification:

- StudioBinder — Script Breakdown Software: https://www.studiobinder.com/script-breakdown-software/
- StudioBinder — Scriptwriting Software: https://www.studiobinder.com/scriptwriting-software/
- StudioBinder — Storyboard Creator: https://www.studiobinder.com/storyboard-creator/
- StudioBinder — Shot List Software: https://www.studiobinder.com/shot-list-software/
- StudioBinder — Free Writing Software: https://www.studiobinder.com/free-writing-software/
- StudioBinder — Storyboard Tools: https://www.studiobinder.com/storyboarding-tool/
- Celtx — Storyboard: https://www.celtx.com/product/story-development/storyboard/
- Celtx — Pre-production: https://www.celtx.com/product/pre-production/
- Celtx — Scheduling: https://www.celtx.com/product/pre-production/schedule/
- Celtx Help — Breakdown: https://support.celtx.com/hc/en-us/articles/216581678-Breakdown
- Filmustage Help — Features Summary: https://help.filmustage.com/en/articles/8282284-features-summary
- Filmustage Help — Stripboard: https://help.filmustage.com/en/articles/15287641-what-is-the-stripboard
- Filmustage Help — Scheduling: https://help.filmustage.com/en/articles/11072778-what-is-the-scheduling-tab
- Filmustage Help — Script Tab: https://help.filmustage.com/en/articles/11008507-what-is-the-script-tab
- Blackmagic Design — DaVinci Resolve Collaboration: https://www.blackmagicdesign.com/products/davinciresolve/collaboration

The product decisions in this PRD intentionally go beyond those references where needed to implement the OpenFrame vision: particularly the local-first model, exchange packages, the deliberately unstructured Idea Vault, the lightweight visual Story Board, and the strict anti-feature-bloat boundary.
