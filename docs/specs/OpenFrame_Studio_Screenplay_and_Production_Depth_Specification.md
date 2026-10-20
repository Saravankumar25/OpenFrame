# OpenFrame Studio — Practical Final Draft Migration + Indie Production Engine Specification

> **Repository integration (2026-10-05).** This is **Revision 1** to the PRD, FSD, UX/UI and Domain specifications,
> promoted from the supplied `OpenFrame_New_Spec_Revised.md` with its content unchanged. Where it differs from those
> specifications for the areas it covers, this document wins (see [docs/README.md](../README.md) §1).
> Three points conflict with **Accepted** ADRs and are resolved by **Proposed** ADRs; until the owner accepts them the
> ADRs below remain in force for code: dark mode and themes (§44, §47A) vs ADR-0012 §7, resolved by ADR-0014;
> frozen and alphanumeric scene numbers (§12) vs ADR-0011, resolved by ADR-0015; Indic-script PDF output (§19, §45)
> vs ADR-0008, resolved by ADR-0016.
> §47A sits after §62 for historical reasons; its numbering (47A.x) is cited elsewhere and stays stable.
> Section 1 describes the provenance of the three supplied sources (manual, Final Draft 13 table of contents,
> StudioBinder call-sheet example). Implementation: [docs/engineering/32](../engineering/32-screenplay-engine-contract.md),
> [33](../engineering/33-production-depth-contract.md), [30](../engineering/30-appearance-themes.md).
> Execution: [docs/implementation/masterplan.md](../implementation/masterplan.md) milestones M2 to M7.

**Version:** 1.0  
**Prepared:** 2026-10-02  
**Purpose:** A practical implementation specification for expanding OpenFrame's screenplay engine to the level required for comfortable migration from Final Draft, without turning OpenFrame into a complicated word processor or a studio-management system.

---

## 1. Source basis and evidence note

This specification compares the supplied OpenFrame product manual, the supplied Final Draft 13 User Guide table of contents, and the supplied StudioBinder call-sheet example.

### Supplied sources

1. **OpenFrame Complete Product Feature Manual**
   - Defines OpenFrame as a local-first filmmaking workspace progressing from Idea Vault → Beat Board → Screenplay → Breakdown → Production → Schedule → Call Sheets.
   - Defines the Screenplay workspace as the professionally strict writing environment and specifies automatic screenplay formatting, drafts, review, locking/revisions, import/export, title page and printing.
   - Its current-status table marks Screenplay, Breakdown, Production, Schedule, Call Sheets, templates and related creative modules as **In development**, so this file is a requirements/upgrade plan rather than a claim that every feature is already shipped.

2. **Final Draft 13 User Guide — supplied `Pasted markdown(5).md`**
   - The uploaded file is a **table of contents**, not the body of the manual.
   - It establishes coverage of Proofing, Font, Script Elements, Title Page, Saving, Printing, Tracking, Views, Formatting, Paragraphs, Page Setup, Production/Scene Numbers, Revisions, Lock Pages, Tags, Beat Board, Outline Editor, Navigator, Panels, Header/Footer, Reformatting, SmartType, Shortcuts and Language.
   - Because the supplied Final Draft source is a TOC only, detailed behavior below is stated as an OpenFrame implementation requirement where appropriate rather than falsely presented as directly verified Final Draft behavior.

3. **StudioBinder Call Sheet template**
   - The supplied five-page PDF shows production contacts, general call, day/date/weather, sunrise/sunset, crew/shooting/meal calls, locations, parking, nearest hospital, scene schedule, talent, atmosphere/background, department notes, advance schedule, crew by department, walkie channels and extras.
   - OpenFrame should use it as a **structural reference**, not copy the visual template.

---

# 2. Product rule: deep engine, simple surface

OpenFrame should **not** copy Final Draft's full ribbon UI, every Word-like control or every niche feature.

The target is:

> **Final Draft migration depth where it affects trust, compatibility and professional writing — with OpenFrame's simpler interaction model.**

A new writer should still be able to:

**Open screenplay → type → press Enter → keep writing.**

A Final Draft user should be able to:

**Import FDX → inspect it → keep writing → revise → export reliable FDX/PDF without feeling that important structure disappeared.**

Advanced capability belongs in the **engine**, but should appear through progressive disclosure.

---

# 3. Executive implementation matrix

| Area | Priority | Decision |
|---|---:|---|
| Standard screenplay element model | P0 | Keep and deepen |
| Automatic element progression | P0 | Keep and deepen |
| Professional pagination/layout engine | P0 | Major expansion |
| Dialogue/parenthetical/character continuation | P0 | Add/deepen |
| Scene numbering | P0 | Deepen |
| Production revision colors | P0 | Keep and make reliable |
| Production/page locking | P0 | Add a simple serious implementation |
| Title page | P0 | Keep and deepen |
| FDX import/export fidelity | P0 | Major engineering priority |
| Fountain import/export fidelity | P0 | Strengthen |
| PDF/DOCX compatibility | P0 | Strengthen and test |
| Import preview/warnings/report | P0 | Keep and make trustworthy |
| Smart structural suggestions | P0 | Add |
| Character/location/scene-intro completion | P0 | Add |
| Custom screenplay elements | P0 | Add behind Advanced |
| Find/Replace | P0 | Keep |
| Scene navigator | P0 | Keep; add useful configuration |
| Header/footer | P1 | Add |
| Page setup presets | P0 | Add/deepen without Word complexity |
| Printing/export preview | P0 | Keep and strengthen |
| Custom keyboard shortcuts | P1 | Add |
| Basic proofing + language dictionaries | P1 | Add |
| Multilingual screenplay text/Unicode | P0 | Add/verify at engine level |
| Beat Board card display customization | P1 | Add |
| Breakdown category/tag colors | P1 | Add |
| Breakdown bulk/quick operations | P1 | Add |
| Schedule timing/travel/availability | P1 | Add |
| DOOD report | P1 | Add |
| Indie-focused call sheet | P0 | Major expansion |
| Lightweight budget | P1 | Keep and improve |
| Templates | P1 | Define exact starter inventory |
| Export branding | P1 | Add |
| Sample scripts | P1 | Add only with copyright-safe policy |
| Full Final Draft ribbon/classic UI | — | Do not implement |
| WGA-specific tooling | — | Defer |
| Speech-control stack | — | Defer |
| Large proofing/Grammarly stack | — | Defer |
| Macro system | — | Defer |
| Full studio accounting | — | Do not implement |
| Full live collaboration | — | Do not implement |

---

# 4. Screenplay engine architecture

## 4.1 Structured document model

The screenplay must be a **structured document**, not a rich-text blob with screenplay formatting painted on top.

Each paragraph should have at minimum:

- stable paragraph ID
- scene ID where applicable
- element type
- text
- position/order
- revision metadata
- import/export metadata when necessary
- comment/annotation anchors where applicable

Visual formatting is generated from these semantics.

### Why this is mandatory

Migration quality depends on preserving meaning, not merely making text look similar.

Example:

```text
Character
Dialogue
```

must remain two structured elements.

It must never become:

```text
Action paragraph with centered text
```

just because it looks visually similar.

---

# 5. Standard screenplay elements

The current OpenFrame manual defines:

- Scene Heading
- Action
- Character
- Parenthetical
- Dialogue
- Transition
- Shot
- General Note

Keep these as the core set.

## 5.1 Each element needs a real rule profile

Every element should have:

- display style
- margins/indentation preset
- alignment
- spacing before/after
- line-wrap rules
- next-element rule
- page-break rules
- continuation rules
- import/export mapping
- optional production semantics

### Scene Heading

Support common heading forms such as:

- `INT.`
- `EXT.`
- `INT./EXT.`
- `I/E`

The heading remains structured scene data. Scene number is separate metadata.

### Character

Character cues should be canonicalized.

For example:

```text
ARJUN
ARJUN (V.O.)
ARJUN (O.S.)
```

should not accidentally create three unrelated story characters.

### Parenthetical

Keep as a real screenplay element.

It should be contextually available in dialogue flow and paginate intelligently.

### Dialogue

Remain structurally tied to the preceding character cue and support continuation across pages.

### Transition

Remain distinct from Action and survive FDX round-trip.

### Shot

Keep optional and production-aware.

A writer should never be forced to think about shot data during ordinary drafting.

### General Note

Remain outside clean screenplay output by default.

---

# 6. Automatic element progression

The current manual already specifies automatic progression. Make it robust.

Expected defaults:

- Scene Heading → Enter → Action
- Character → Enter → Dialogue
- Parenthetical → Enter → Dialogue
- Action → Enter → Action
- Dialogue → Enter → contextually appropriate next element
- Transition → Enter → appropriate next element

The user must always be able to:

- pick an element manually
- use a keyboard shortcut
- convert the current paragraph
- override the next suggested element

### Simplicity rule

The editor predicts **structure**, not prose.

---

# 7. Smart structural suggestions / SmartType-style assistance

This is one of the highest-value Final Draft-inspired features because it removes repetitive typing while remaining invisible when not needed.

## 7.1 Suggestions to support

### Scene-heading terms

- `INT.`
- `EXT.`
- `INT./EXT.`
- `I/E`
- project-specific/custom variants

### Character names

Build suggestions from:

- screenplay character cues
- Beat Board character directory
- cast-linked characters

### Locations

Suggest:

- locations already in the screenplay
- story locations
- production locations
- recently used locations

### Scene-intro/time values

Suggest common structural values such as:

- DAY
- NIGHT
- LATER
- CONTINUOUS
- SAME
- MOMENTS LATER

These remain optional.

### Parenthetical handling

The primary assistance should be selecting the **Parenthetical element**, not generating prose.

A recently used/custom parenthetical list is acceptable as lightweight convenience.

## 7.2 Suggestion behavior

Required:

- typing filters the list
- Enter accepts
- Esc dismisses
- arrows navigate
- mouse click accepts
- “Create New” allows a new project value where appropriate
- one Enter on a character name moves on to Dialogue, like Final Draft (owner decisions 2026-10-08): on “Create New” it adds the character first, on a known name chosen from the list it fills the name first; Esc keeps the typed name without adding it, and the next Enter then moves on as usual; Undo takes back the Dialogue step first, then the new character (if one was added), then the name; Tab never accepts a suggestion and keeps its element meaning

Never silently replace an existing value because it resembles a suggestion.

---

# 8. Professional formatting and pagination — P0

This is the biggest screenplay-engine expansion.

The engine must handle screenplay formatting automatically while allowing limited advanced configuration.

## 8.1 Controlled page presets

Support:

- standard screenplay preset
- Letter
- A4
- project-specific saved preset

Advanced settings may expose:

- paper size
- page margins
- font
- font size
- line spacing
- header/footer
- page number behavior
- title-page behavior
- scene-number behavior
- revision display

Do not provide arbitrary Word-style formatting in the normal editor.

---

# 9. Pagination rules that must be first-class

## 9.1 Keep-together rules

Test and handle:

- scene heading should not be stranded when moving the block is possible
- character cue should stay with its dialogue when practical
- parenthetical should remain sensibly attached to the surrounding dialogue
- short orphan lines should be handled consistently

## 9.2 Dialogue continuation

Support production-quality continuation behavior for dialogue that crosses pages.

The underlying engine must know the difference between:

- one dialogue block split by a page boundary
- two separate dialogue blocks

Continuation indicators must be generated from structure rather than typed as ordinary screenplay text.

## 9.3 Intentional page breaks

Provide:

**Insert Page Break**

with:

- add
- remove
- undo
- export/import preservation where supported

Store it as a real page-break directive, not a pile of newline characters.

## 9.4 Deterministic reflow

The same document state should produce the same pagination.

Any text change must recalculate pages predictably.

This is foundational for:

- PDF
- printing
- revisions
- page locking
- production revisions
- migration comparison

---

# 10. Page locking and production locking

The supplied Final Draft TOC explicitly includes Lock Pages, Lock A Pages and locking tools. OpenFrame does not need to copy the whole locking menu, but it needs a credible equivalent.

## 10.1 OpenFrame's existing lock

The manual already defines:

**Lock as Shooting Draft**

and:

**Start Revision**

Keep this.

## 10.2 Add page-level production locking

After a shooting draft is locked:

- its pagination becomes the production baseline
- later edits go through a new revision
- changed pages are identified
- revision data records affected scenes/pages
- unchanged production pages should remain stable where the revision model permits

## 10.3 Keep the interaction simple

Target:

```text
Lock Shooting Draft
       ↓
Start Revision
       ↓
Revision A
       ↓
Changed pages
```

Do not reproduce a large collection of niche locking commands unless real users need them.

---

# 11. Revision colors — P0

OpenFrame already defines:

- Blue
- Pink
- Yellow
- Green
- Custom

Keep that design.

## Required data

Each revision records:

- label
- color
- source draft
- date
- reason
- changed scenes
- changed pages

## Rendering rule

Revision color represents a **production revision**, not normal writing color.

Clean screenplay output:

- revision marks OFF by default

Production revision output:

- revision marks ON when requested

---

# 12. Scene numbering — P0

Keep the current stable-identity model and deepen it.

Required:

- automatic numbering
- generated number separate from database identity
- renumber after script reorder
- import existing scene numbers where present
- export scene numbers
- support alphanumeric production numbering where needed
- warn when a scene already has production dependencies

Example:

```text
Stable scene ID: 8b4e...
Displayed scene number: 27
```

Changing 27 → 25 must not break:

- breakdown
- catalog links
- shots
- storyboard
- schedule
- call sheets

---

# 13. Title page — P0

OpenFrame's manual already specifies Figure 100 Title Page.

Required fields:

- title
- author/creator
- contact name
- email
- phone
- address
- draft name
- revision label
- date
- optional representation/contact block

Provide:

- preview
- export inclusion toggle
- save

Avoid arbitrary drag-anywhere page design.

---

# 14. Header and footer — P1

The supplied Final Draft TOC includes Header/Footer content, options and placeholders.

Add a controlled OpenFrame version.

Support:

- header text
- footer text
- page number
- project title placeholder
- draft/revision placeholder
- first-page/title-page behavior

Example:

```text
BLACK RAIN
Draft 7 — Page 27
```

Keep this preset-driven rather than Word-like.

---

# 15. Page setup — P0

Support only what a screenplay writer realistically needs:

- paper size
- page-number start
- title-page behavior
- header/footer
- scene numbering
- revision display
- approved font/layout presets

Avoid:

- arbitrary tables
- floating objects
- multi-column rich text
- paragraph borders
- text boxes
- unrestricted tab systems

---

# 16. Scene navigator

OpenFrame already has a scene navigator.

Improve it with optional columns:

- Scene #
- Heading
- Page
- Short description
- Comments
- Revision/change status
- Breakdown status
- Production status

Default:

```text
Scene # | Heading
```

Advanced:

```text
Scene # | Heading | Page | Notes | Revision | Production
```

Allow the user to choose visible columns.

This is the right amount of “Navigator 2.0-like” depth without creating a second application inside the screenplay editor.

---

# 17. Custom screenplay elements — P0 behind Advanced

The supplied Final Draft TOC includes creation/modification of custom elements.

OpenFrame should support this, because migration may encounter custom FDX semantics.

A custom element should have:

- name
- optional shortcut
- display preset
- spacing/indent preset
- next-element behavior
- import/export identity

### Critical migration rule

If FDX contains an unknown/custom element:

```text
Custom Final Draft element detected: TELEPHONE

[Preserve as Custom]
[Map to existing element]
[Review]
```

Never silently convert it to Action.

---

# 18. Find and Replace

OpenFrame already defines:

- Find
- Replace
- Replace All
- Match case
- Whole word
- Next/Previous
- Undoable changes

Keep those.

Add only light scope control:

- Current scene
- Entire screenplay

Do not put a regular-expression editor in the default UI.

---

# 19. Proofing and multilingual writing

Final Draft's TOC includes Proofing, Spell-Checking, Spelling Options, Thesaurus, Grammarly and foreign-language spell-checking.

Select the useful subset.

## P0 — Unicode/multilingual screenplay engine

Must support:

- Telugu
- Hindi
- Tamil
- Malayalam
- Kannada
- English
- mixed-language scripts

as Unicode-safe examples.

The same text must survive:

- typing
- save/reload
- PDF
- FDX
- Fountain
- DOCX

without corruption.

## P1 — spell checking

Support:

- project language
- spell-check toggle
- misspelling underline
- personal dictionary
- available language dictionaries

## P1 — mixed-language scripts

Allow primary + secondary proofing languages.

### Defer

- full thesaurus
- Grammarly integration
- large grammar rewriting system
- AI proofreading as a dependency

---

# 20. Custom keyboard shortcuts — P1

Make the manual's “can be customised later” statement a real feature.

## Settings → Keyboard Shortcuts

Display:

| Command | Default | Current | Conflict |
|---|---|---|---|
| New Project | Ctrl+N | Ctrl+N | — |
| Find | Ctrl+F | Ctrl+F | — |
| Scene Heading | Ctrl+1 | Ctrl+1 | — |
| Action | Ctrl+2 | Ctrl+2 | — |
| Character | Ctrl+3 | Ctrl+3 | — |

Required:

- remap
- remove
- restore individual default
- restore all defaults
- conflict detection
- multiple shortcuts per command where practical

Every command must also retain an on-screen route.

---

# 21. Beat Board Scene Card redesign requested by product owner

The existing Beat Board is intentionally lightweight. Keep that philosophy while increasing information capacity.

## 21.1 Larger scene description

Scene Cards should have:

- short summary
- long description
- notes
- attachments
- optional metadata

The long description should be large enough to hold meaningful scene-development material.

## 21.2 User-defined minimized card content

Add:

**Beat Board → Card Display Settings**

Checkboxes:

- Scene Heading
- Short Description
- Long Description Preview
- Notes
- Attachments
- Character Count
- Location
- Day/Night
- Comments
- Revision/Change
- Breakdown Status
- Shot/Storyboard Status

The user decides what is visible while minimized.

## 21.3 Presets

Offer:

- Minimal
- Story
- Production
- Custom

Custom allows arbitrary checkbox combinations.

Expanded view may show all configured information.

### Critical rule

Display settings never change underlying scene data.

---

# 22. Breakdown — preserve current architecture

OpenFrame already specifies:

- eleven default categories
- script highlighting
- manual entry
- Suggest Elements
- Accept/Reject
- Catalog reuse
- source-draft tracking
- reconciliation after script change
- Needs Review

Keep this architecture.

---

# 23. Breakdown category colors — P1

Yes. Different colors for different categories are useful for visual scanning.

Recommended category identities:

- Cast
- Extras/Background
- Location/Set
- Props
- Wardrobe
- Vehicles
- Hair/Makeup
- Special Effects
- VFX
- Sound
- Animals

### Accessibility

Never use color alone.

Each category must also have:

- text label
- icon/symbol
- accessible tooltip/label

### Customization

Allow color customization in Advanced Settings.

Normal users do not need to configure anything.

---

# 24. Breakdown efficiency upgrades

Add:

- accept selected
- reject selected
- quick reuse
- category filters
- only unreviewed
- only suggested
- only missing
- scene completion status
- source-phrase jump
- catalog-item → all-scene jump
- concise breakdown report

Keep the rule:

**Suggestions never become truth until accepted.**

---

# 25. Schedule — practical depth, not an optimizer

The current OpenFrame schedule foundation is good:

- Unscheduled pool
- Stripboard
- List
- Calendar
- shooting days
- off days
- break markers
- estimates
- day totals
- conflict warnings
- grouping suggestions
- source tracking
- stale handling
- call-sheet refresh

Add only the practical missing layer.

---

# 26. Schedule upgrades required for indie makers

## 26.1 Day timeline model

Support:

- crew call
- shooting start
- target wrap
- scene start time
- scene duration
- meal/break duration
- travel/company move duration

Example:

```text
07:00 Crew Call
08:00 Scene 1 — 60m
09:00 Scene 3 — 90m
10:30 Break — 30m
11:00 Scene 9 — 60m
12:00 Company Move — 30m
12:30 Scene 2 — 120m
14:30 Lunch — 60m
15:30 Scene 11 — 60m
16:30 Scene 6 — 120m
18:30 Estimated Wrap
```

This should be an estimate, not a false promise of exact real-world timing.

## 26.2 Travel/company-move rows

Support:

- Travel
- Company Move
- Meal Break
- Custom Break

These affect estimated day duration.

## 26.3 Availability warnings

Use cast/location records where entered.

Warn about:

- cast unavailable
- location unavailable
- required location missing
- day exceeds target
- travel/move makes the day exceed target
- call sheet stale
- production source changed

Warnings remain advisory.

---

# 27. Schedule filters

Add:

- location
- cast
- day/night
- scene length
- scheduled/unscheduled
- breakdown complete/incomplete
- shooting day
- revision/change status

No automatic scene optimizer is required.

---

# 28. DOOD report — P1

**DOOD = Day Out of Days.**

Add a report derived from cast assignments and the shooting schedule.

## Layout

Rows:

- cast members

Columns:

- shooting days / dates

Cells:

- working/non-working state
- configurable industry-style codes where required

Example:

```text
Actor      Day 1  Day 2  Day 3  Day 4
Arjun        W      W      -      W
Maya         W      -      W      W
Ravi         -      W      W      -
```

Required:

- select cast
- select date/day range
- automatic derivation
- manual correction
- PDF export
- spreadsheet export

Do not build contracts/payroll around it.

---

# 29. Call Sheets — major practical upgrade

The current manual's Call Sheet editor is conceptually correct but needs more production information to become genuinely useful.

Use the supplied StudioBinder PDF as the structural reference.

## 29.1 Production header

Support:

- project/production name
- production company
- production address
- producer
- 1st AD
- director
- main production contact

## 29.2 Day information

Support:

- shooting day number
- total days
- date
- general call
- crew call
- shooting call
- breakfast/meal call
- lunch call
- estimated wrap
- weather
- sunrise
- sunset

## 29.3 Locations

For each location:

- set/location name
- address
- parking
- nearest hospital
- emergency contact if available
- meeting/access instructions
- location notes

Unknown values should show:

`Needs input`

not invented values.

## 29.4 Shooting schedule table

Each scene row:

- start time
- scene number
- scene heading
- short description
- D/N
- cast
- location
- pages
- estimated duration

Optional rows:

- travel
- company move
- meal break
- custom note

## 29.5 Talent

Support:

- actor
- character
- ID
- status
- pickup time
- call time
- hair/makeup
- contact
- notes

## 29.6 Extras/Atmosphere

Support:

- group/description
- quantity
- call time
- on-set time
- notes

## 29.7 Department notes

Small structured notes for:

- Set Dressing
- Props
- Costumes
- Sound
- Makeup/Hair
- Art
- Camera
- Production
- Custom department

## 29.8 Advance Schedule

Optional section for:

- next-day location
- expected cast
- special props
- wardrobe
- advance preparation

## 29.9 Crew

Group by department:

- Production
- Direction
- Camera
- Grip
- Lighting
- Sound
- Art
- Makeup/Hair
- Costume
- Transport
- Other

Per person:

- name
- role
- department
- call
- optional contact

## 29.10 Communication

Optional:

- walkie channels
- emergency contact
- production contact
- special communication note

## 29.11 Practical notes

Support:

- parking
- meeting point
- transport
- meals
- toilets
- access instructions
- safety
- weather notes
- special instructions
- reference images/attachments

---

# 30. Call-sheet UX — simple by default

Do not show every empty section.

Default:

1. Production header
2. Day/time
3. Locations
4. Scene schedule
5. Talent
6. Practical notes

Then:

**+ Add Section**

Optional:

- Weather
- Parking
- Emergency
- Extras
- Department Notes
- Advance Schedule
- Crew
- Walkie Channels
- Reference Images
- Special Notes

---

# 31. Call-sheet source model

Keep:

**Shooting Day → Create Call Sheet**

Automatically prefill:

- day/date
- scenes
- cast
- locations
- page counts
- estimates
- production contacts
- existing notes

Editing the call sheet must never change:

- screenplay
- breakdown
- schedule

---

# 32. Call-sheet stale/update model

Keep the manual's:

- Source Changed
- Needs Refresh
- Update Preview
- Finalized snapshot

Improve the update preview:

```text
3 scenes changed
1 cast call changed
1 location changed
Day date unchanged

[Review Changes] [Update]
```

Finalized sheets remain stable historical snapshots.

---

# 33. Lightweight budget

OpenFrame already specifies a small budget view. Keep its boundary.

Current categories:

- Cast
- Crew
- Locations
- Equipment
- Art/Props
- Travel/Transport
- Food
- Post/Other
- Contingency

## Practical upgrade

Add:

- Planned
- Actual
- Notes

Example:

| Category | Item | Planned | Actual |
|---|---|---:|---:|
| Equipment | Camera | ₹8,000 | ₹7,500 |
| Travel | Fuel | ₹6,000 | ₹6,800 |

Show:

- category totals
- overall planned total
- actual total
- remaining/difference

Do not add:

- payroll
- tax
- invoices
- purchase orders
- accounting ledger
- ERP

---

# 34. Templates

## Current documented state

The manual defines a Template Browser and says it contains:

- project starters
- Beat Board starters
- call-sheet layouts
- shot-list layouts
- moodboards
- simple budgets
- user-created templates

It says **“A small number of useful starters.”**

### Exact current count

**Not specified in the supplied manual.**

Therefore the project must not claim an existing exact template count until one is explicitly defined and implemented.

## Recommended release inventory: 8 built-ins

1. Feature Film Screenplay
2. Short Film Screenplay
3. Episodic / Series
4. Simple Beat Board
5. Indie Call Sheet
6. Shot List
7. Moodboard
8. Lightweight Budget

Also support:

- Preview
- Use Template
- Save as Template
- Rename custom template
- Delete custom template
- independent copy behavior
- permanent Blank Project path

---

# 35. Copyright-safe sample screenplay folder

A built-in sample-script library is useful for new users and for testing.

However:

> A screenplay being publicly viewable online does not automatically mean OpenFrame can redistribute its text.

## Safe sources

Use only:

- OpenFrame-authored scripts
- public-domain material
- explicitly licensed material
- material for which OpenFrame has redistribution permission

## Recommended built-in folder

```text
OpenFrame
└── Samples
    ├── Example Feature
    ├── Example Short
    ├── Example Scene
    └── FDX Interchange Test
```

Make them fictional and OpenFrame-authored where possible.

Include test coverage for:

- ordinary dialogue
- heavy action
- revisions
- custom elements
- multilingual text
- FDX round-trip

Do not package random screenplays downloaded from the internet.

---

# 36. FDX import/export — P0 engineering priority

Migration trust will depend heavily on this.

Supported screenplay interchange:

- FDX
- Fountain
- PDF
- DOCX
- TXT/pasted text

## 36.1 FDX import

Preserve where supported:

- scene headings
- action
- character
- parenthetical
- dialogue
- transitions
- shot/custom elements
- scene numbers
- title-page data
- revision metadata

### Import preview

Show:

```text
Imported:
47 scenes
12 characters
109 screenplay elements

Warnings:
2 custom elements
1 ambiguous scene heading
1 unsupported formatting construct
```

### No silent flattening

Unknown constructs must:

- be preserved
- or be explicitly mapped
- or be clearly marked for review

## 36.2 FDX export

Export must:

- preserve element semantics
- preserve scene numbering where selected
- preserve title data where supported
- preserve revisions where supported
- preserve custom elements
- remain re-importable by OpenFrame
- be structurally readable by Final Draft

---

# 37. FDX round-trip suite

Create regression fixtures for:

- basic script
- dialogue-heavy
- parenthetical-heavy
- transitions
- action-heavy
- multilingual
- custom elements
- revision-marked
- scene-numbered production draft
- title page
- explicit page breaks
- dialogue spanning pages
- unusual/legacy FDX constructs

Every fixture should have:

- source file
- expected parsed structure
- expected exported structure
- expected rendered PDF snapshot

---

# 38. Fountain import/export

Keep it lightweight but deterministic.

Preserve:

- scene headings
- action
- character
- parenthetical
- dialogue
- transition
- notes where mapping is available

Warn rather than silently corrupt if semantics are ambiguous.

---

# 39. PDF import

Continue treating PDF as best effort.

Retain:

- preview
- scene detection
- character detection
- page count estimate
- warnings
- safe failure
- original-file preservation

Never claim exact structured fidelity when the source PDF does not support it.

---

# 40. DOCX import/export

Treat DOCX as a compatibility format.

Requirements:

- parse paragraphs into screenplay elements where possible
- preserve readable screenplay content
- avoid pretending arbitrary rich-text layout is screenplay semantics
- warn on ambiguity

The imported result must become structured screenplay data.

---

# 41. Export preview / reliability contract

Use:

```text
Choose source
    ↓
Choose format
    ↓
Generate preview
    ↓
Validate
    ↓
Export
```

Export failures must not mutate the project.

---

# 42. PDF visual-regression tests

For representative screenplay fixtures:

1. generate PDF
2. render to page images
3. compare against expected structure/layout
4. verify:
   - page count
   - margins
   - scene headings
   - dialogue blocks
   - title page
   - page numbers
   - continuation behavior
   - revision marks
   - headers/footers
   - no clipping
   - no overlap

The goal is **document trust**, not maximum formatting configurability.

---

# 43. Branding for non-screenplay exports

User requirement:

> Every export/download should carry OpenFrame branding except screenplay documents.

## Screenplay outputs — no branding

Keep these clean:

- screenplay PDF
- screenplay FDX
- screenplay Fountain
- screenplay DOCX

## Production/non-screenplay outputs — subtle branding

Brand:

- call-sheet PDF
- schedule PDF
- schedule spreadsheet
- breakdown reports
- production reports
- DOOD
- budget report
- shot-list PDF
- storyboard sheet
- moodboard sheet
- other generated production deliverables

Use:

- small OpenFrame logo
- “Created with OpenFrame Studio” (superseded 2026-10-20: the owner's PDF change specs use “Created using OpenFrame”)
- header/footer or unobtrusive watermark

For spreadsheets, use workbook/first-sheet metadata or footer rather than putting a logo into every row.

For project packages/backups, use an OpenFrame-branded manifest/README rather than modifying user-owned files.

---

# 44. Dark mode

The supplied OpenFrame manual does not specify dark mode.

It is not migration-critical, but it is useful for writing comfort.

Implement:

- Light
- Dark
- System

No full theme editor is needed.

The printed screenplay appearance remains controlled by document/export settings.

---

# 45. Three-layer language model

## Layer 1 — script language: P0

Unicode-safe screenplay text for regional and international languages.

## Layer 2 — proofing language: P1

Spell-check language selection and dictionaries.

## Layer 3 — application UI localization: P1/P2

Localization-ready architecture and translated UI as resources allow.

The engine must handle multilingual content before large UI-localization work begins.

---

# 46. What NOT to implement from Final Draft

The supplied Final Draft TOC covers many things. OpenFrame should deliberately omit or defer these unless actual user evidence justifies them:

## Full Ribbon UI

Do not copy.

## Classic View

Do not copy.

## Full Quick Access Toolbar architecture

A small command bar is enough.

## WGA-specific tools

Not central to OpenFrame's indie workflow.

## Speech Control

Useful to some users, but not migration-critical.

## Extensive Emoji/Symbol UI

OS/Unicode input is sufficient.

## Full Thesaurus subsystem

Not necessary to the core product.

## Grammarly integration

Not necessary for core screenplay migration.

## Large writing-goal/sprint system

Optional productivity feature, not a migration requirement.

## Large macro system

Adds complexity and support cost.

## Word-level arbitrary formatting

Not the goal.

---

# 47. What OpenFrame should cover through existing concepts

| Final Draft concept | OpenFrame approach |
|---|---|
| Beat Board | Beat Board |
| Outline Editor | Beat Board + Outline |
| Structure Lines | Story relationships; do not create a second duplicate system unless needed |
| Script Navigator | Scene Navigator + configurable columns |
| ScriptNotes | OpenFrame Notes + Comments |
| Tags | Breakdown/Catalog instead of another disconnected production database |
| Multiple drafts | OpenFrame Drafts |
| Revision workflow | OpenFrame Revisions |
| Production tagging | Breakdown |
| Production reports | Reports + DOOD |
| Call sheets | Schedule → Call Sheet |
| Custom elements | OpenFrame custom element engine |
| Sample scripts | OpenFrame-authored/public-domain/licensed |
| Collaboration | File-based Exchange/Review/Response |
| Page locking | Simple OpenFrame production/page lock |

---

# 48. Screenplay editor UX target

Normal writing mode should remain visually quiet.

Default visible:

- screenplay page
- element indicator
- scene navigator
- draft name
- save state
- Find
- Focus mode

Advanced one-click access:

- revisions
- title page
- page setup
- header/footer
- custom elements
- shortcut configuration
- proofing
- detailed pagination options

The writer should feel:

> “I am writing a screenplay.”

not:

> “I am configuring a desktop publishing system.”

---

# 49. Screenplay-to-production integrity

Keep the existing source-of-truth architecture:

```text
Screenplay draft
      ↓ explicit production source
Breakdown
      ↓
Catalog
      ↓
Schedule
      ↓
Call Sheet
```

If a new screenplay revision exists, production stays on the old source until explicitly updated.

This must remain independent of Final Draft migration work.

---

# 50. Migration definition of done

A Final Draft user should be able to:

1. Import an FDX containing standard screenplay elements.
2. Import an FDX containing revisions.
3. Import an FDX containing scene numbers.
4. Import an FDX containing custom elements.
5. Inspect all warnings before import.
6. Edit the script normally.
7. Export FDX.
8. Re-open the exported FDX in Final Draft.
9. Re-import that FDX into OpenFrame.
10. Export clean PDF.
11. Compare the resulting structure to the original.

A migration is not considered successful merely because the text is visible.

---

# 51. Acceptance criteria — screenplay engine

Do not call the screenplay engine production-ready until:

- all standard OpenFrame elements are editable
- automatic element progression is reliable
- structural suggestions are reliable
- FDX import is safe
- FDX export is structurally valid
- FDX round-trip tests pass
- pagination is deterministic
- continuation handling is correct
- manual page breaks work
- scene numbers are deterministic
- title page works
- revisions work
- revision colors work
- production/page locking works
- PDF export works
- print preview matches PDF output
- custom elements never silently flatten
- multilingual text survives save/import/export
- shortcuts can be customized
- shortcut conflicts are detected
- imported scripts produce a useful report
- export failures leave the source untouched

---

# 52. Acceptance criteria — Beat Board

- scene descriptions hold substantially more information
- attachments remain linked
- minimized card display is configurable
- presets are available
- Custom mode exists
- expanded view is readable
- display settings are separate from data
- changing card layout never modifies screenplay text

---

# 53. Acceptance criteria — Breakdown

- eleven default categories remain
- category colors are visible
- text/icons accompany every color
- suggestions can be reviewed
- catalog reuse remains reliable
- source phrase remains traceable
- changes create review state
- no production data is silently deleted
- bulk useful actions work
- reports export
- filters work at larger project sizes

---

# 54. Acceptance criteria — Schedule

- scenes move between days
- Unscheduled remains reliable
- day totals update
- start time + duration is supported
- break/travel/company-move rows work
- cast/location conflicts warn
- availability warnings work
- source changes create review state
- schedule exports are readable
- no automatic optimizer moves scenes without explicit approval

---

# 55. Acceptance criteria — DOOD

- cast selection works
- shooting-day columns generate
- status cells derive from schedule
- manual overrides work
- date/day ranges work
- PDF export works
- spreadsheet export works
- report remains read-only to screenplay/breakdown

---

# 56. Acceptance criteria — Call Sheets

A call sheet must be capable of representing, when information exists:

- production identity/contact
- producer/AD/director
- general call
- crew call
- shooting call
- day/date
- weather
- sunrise/sunset
- locations
- parking
- nearest hospital
- scene schedule
- start times
- scene numbers
- scene descriptions
- D/N
- cast
- location
- pages
- estimated duration
- travel/moves
- talent calls
- pickup
- HMU
- extras/atmosphere
- department notes
- advance schedule
- crew by department
- walkie channels
- practical notes
- emergency contact
- attachments

Optional sections remain hidden unless requested.

---

# 57. Acceptance criteria — exports

Every generated document must:

- have the correct file extension
- open successfully
- contain the intended source data
- identify the correct source/version where applicable
- exclude private notes unless explicitly permitted
- not silently omit supported information
- give a recoverable error on failure

Every non-screenplay export receives subtle OpenFrame branding.

Every screenplay export remains clean/unbranded.

---

# 58. Recommended implementation order

## Phase 1 — Screenplay trust

1. structured element model
2. pagination/layout
3. continuation rules
4. scene numbering
5. title page
6. FDX import
7. FDX export
8. PDF/print generation
9. revision colors
10. production/page locking
11. custom elements
12. structural Smart suggestions

## Phase 2 — Writer comfort

13. custom shortcuts
14. Unicode/multilingual support
15. spell checking
16. configurable scene navigator
17. headers/footers
18. page setup presets
19. dark/System/Light themes

## Phase 3 — Indie production depth

20. Beat Board card customization
21. breakdown category colors
22. breakdown quick/bulk actions
23. schedule timing/travel/availability
24. DOOD
25. call-sheet expansion
26. lightweight budget Planned/Actual
27. branded production exports

## Phase 4 — Templates and test assets

28. eight initial templates
29. copyright-safe sample scripts
30. migration fixture corpus
31. FDX conformance suite
32. screenplay PDF visual regression
33. production-document visual regression

---

# 59. Critical anti-complexity rule

Do not add a feature merely because Final Draft has it.

Use this test:

### Add it when it:
- prevents migration loss,
- increases professional trust,
- removes repetitive work,
- improves production safety,
- or materially helps an indie filmmaker.

### Hide it behind Advanced when it:
- is useful only occasionally,
- changes document configuration,
- is production-specific,
- or adds technical detail.

### Defer it when it:
- mainly adds feature-count marketing,
- recreates Word,
- adds a large dependency,
- or has no clear indie workflow benefit.

---

# 60. Final product target

The desired product is:

**Final Draft-depth screenplay engine**
+
**simple writing experience**
+
**OpenFrame's end-to-end filmmaking workflow**
+
**strong import/export interoperability**
+
**local ownership**
+
**practical indie production tools**

The winning design principle is not:

> “Have more controls than Final Draft.”

It is:

> **“Handle professional complexity internally so the filmmaker does not have to.”**

---

# 61. Direct answers to the requested additions

## Templates

**Current documented state:** template functionality is specified, but the supplied manual does not state an exact number of built-in templates.

**Recommended release:** 8 built-ins across screenplay/story/production workflows.

## Internet screenplay samples

**Do not redistribute random internet screenplays.** Public availability is not the same as redistribution permission.

Use OpenFrame-created, public-domain, or explicitly licensed scripts.

## Structural suggestion list

**Not specified in the screenplay editor today as a full SmartType-equivalent.**

Add context-aware suggestions for:

- scene-heading prefixes
- characters
- locations
- scene-intro/time terms
- element selection
- recent/custom parenthetical forms

## Custom shortcuts

**Specified as future-customizable in the manual, but should be implemented as a real settings feature.**

Add conflict checking and restore-default controls.

## Title page

**Specified in the manual and should remain P0.**

The current specification includes title, author/creator, contact details and optional draft/revision information.

## Beat Board card descriptions

Increase capacity and make minimized-card fields user-selectable.

## Granular screenplay formatting

This is the largest required improvement:
- deterministic pagination
- continuation behavior
- page breaks
- scene-numbering correctness
- title page
- headers/footers
- page setup presets
- revision-aware pagination
- custom element support
- FDX semantic preservation

## Lightweight budget

**Already specified.** Keep it lightweight, then add Planned/Actual tracking.

## Multilingual support

Make Unicode-safe screenplay writing P0 and spell-check languages P1.

## Dark mode

Not required for migration, but add Light/Dark/System as a small comfort feature.

## DOOD

Add a derived Day Out of Days report from cast + shooting schedule.

## Revision colors

**Already specified.** Blue/Pink/Yellow/Green/Custom. Make the feature production-revision reliable.

## Breakdown tag colors

Add category colors, paired with text/icons for accessibility.

## Import/export reliability

Treat FDX as migration-critical and build a true conformance/round-trip suite.

## Call sheets

The supplied StudioBinder example demonstrates that a truly useful indie call sheet should include significantly more than the current minimal document. Add the production, timing, location, schedule, talent, extras, crew, department and communication information listed in Sections 29–32, while keeping optional sections hidden by default.

---

# 62. Final implementation philosophy

A feature is **not complete** just because:

- a button exists,
- a backend function exists,
- one screenshot looks correct,
- or one export succeeds.

A feature is complete only when:

```text
Discover
   ↓
Use
   ↓
Save
   ↓
Undo
   ↓
Recover
   ↓
Export
   ↓
Re-import / reopen
   ↓
Continue working
```

without losing semantic meaning.

That is the level of depth OpenFrame needs to become **migration-trustworthy** while remaining an approachable indie filmmaking tool.

# 47A. Appearance Theme System — Optional User Themes, Canonical Default Preserved

## 47A.1 Product intent

OpenFrame should support personal appearance choices without turning the application into a visual customization tool.

The existing OpenFrame UI shown in the product design is the **canonical default appearance**. It must remain the default software design and must not be replaced, weakened, recolored, or visually redesigned merely because the theme system is introduced.

The theme system is an **additional layer on top of the existing design**:

```text
Existing OpenFrame appearance
        = Default / factory appearance
                 +
        Optional user-selected theme
```

A user who never opens Appearance Settings should see the same OpenFrame design that exists today.

A user may deliberately select another theme when they prefer a different visual mood.

The target is:

> **Keep the existing OpenFrame design as the trusted default; add professional optional themes without changing the product's information architecture, layout, controls, typography, workflow, or screenplay document appearance.**

---

## 47A.2 Design principles

### Principle 1 — The current design wins

The present OpenFrame palette, spacing, component hierarchy, typography and visual language remain the default.

Do not make the current interface look like one of the new themes merely because the theme engine exists.

### Principle 2 — Themes change appearance, not product behavior

A theme must not change:

- navigation
- labels
- workflows
- feature availability
- data
- screenplay semantics
- production logic
- export structure
- keyboard shortcuts
- permissions
- file paths
- project identity

It changes visual tokens only.

### Principle 3 — Deep theming, zero component edits

A developer adding a new theme must not edit individual components.

A theme should ideally be added by:

1. one small theme declaration containing its seed values;
2. registering its metadata in the theme registry.

No screen-by-screen recoloring should be required.

### Principle 4 — Professional, not novelty-first

Themes are for filmmakers, writers, editors, directors, producers and students.

The built-in palette set should cover:

- long writing sessions
- editing/focused work
- production planning
- calm/low-stimulation work
- warm/cinematic moods
- distinctive creative personalities
- accessibility
- neutral viewing for color-sensitive workflows

Avoid themes that make text difficult to read, make status colors ambiguous, or turn OpenFrame into a gaming/RGB interface.

### Principle 5 — Progressive disclosure

A first-time user should not need to understand tokens, color science, contrast ratios or seed values.

The normal UI exposes:

**Appearance → Mode → Theme**

Technical theme data stays behind the scenes.

---

## 47A.3 Theme modes

OpenFrame provides three appearance modes:

- **Light**
- **Dark**
- **System**

### Light

Uses the theme's light token ladder.

### Dark

Uses the same theme's dark token ladder.

The dark version is derived from the same theme seeds rather than requiring a separate theme definition.

### System

Follows the operating system's light/dark preference using the application's normal OS media-query/event mechanism.

When the OS preference changes while OpenFrame is running, System mode updates automatically without requiring an application restart.

### Important separation

**Theme** and **Mode** are separate settings.

Examples:

```text
Theme: Midnight Reel
Mode: Light
```

and:

```text
Theme: Midnight Reel
Mode: Dark
```

are valid combinations.

A user is never forced to use a theme only in the mode suggested by its name.

---

## 47A.4 Canonical default theme — preserve today's OpenFrame design

The existing OpenFrame design must be represented as the factory/default theme.

### Default behavior

On a new installation:

```text
Theme = OpenFrame Default
Mode  = System
```

where the resolved Light/Dark appearance remains the current OpenFrame design language.

If the current design is already represented by the prototype's `Studio Amber` seed profile, that profile may be retained internally as the seed definition for the canonical default. **Do not change the visible default palette solely to introduce this feature.**

### Migration rule for existing installations

Existing users must not unexpectedly wake up to a different theme.

On first launch after the theme system is introduced:

- preserve the existing appearance as the active/default visual state;
- migrate the stored appearance preference, if any;
- only use the canonical default fallback when no valid preference exists.

If a stored custom/imported theme becomes invalid or unavailable:

```text
Fallback → OpenFrame Default
```

and the user's work/data remain untouched.

---

## 47A.5 Theme architecture

The real UI implementation should be adapted to the application's actual stack, but the architecture should follow one invariant:

> **All UI colors come from semantic theme tokens. Components do not own literal palette colors.**

The current uploaded prototype demonstrates the intended architecture:

- `openframe.css` consumes semantic variables such as `--ink`, `--backdrop`, `--panel`, `--side`, `--accent`, status colors and soft variants.
- `themes.css` defines light/dark token ladders and theme seed blocks.
- `themes.js` applies `data-theme` and `data-mode`, stores preferences locally and updates System mode from `prefers-color-scheme`.

The production implementation must preserve this separation even if the final framework/runtime differs.

### Semantic token examples

At minimum provide semantic roles for:

```text
background
backdrop
panel
surface
surface-secondary
surface-tertiary
well
chrome
text-primary
text-secondary
text-muted
border
border-strong
sidebar
sidebar-secondary
sidebar-text
sidebar-muted
hover
accent
accent-strong
accent-soft
accent-border
accent-focus-ring
paper
paper-text
status-info
status-success
status-warning
status-danger
status-neutral
```

Additional tokens may be added only where a real component semantic requires them.

### No literal component palette values

Component code must not contain literal UI colors such as:

```css
background:#123456;
color:#222;
border:1px solid #ddd;
```

for themeable UI states.

Use:

```css
background:var(--panel);
color:var(--ink);
border-color:var(--line);
```

instead.

Existing hard-coded UI colors should be classified:

1. **themeable UI color** → replace with a semantic token;
2. **intentional fixed product color** → document and retain;
3. **document/export color** → keep in the document/export layer, not the application theme layer.

---

## 47A.6 Seed-based theme model

A new theme should be defined primarily by a small seed set.

Recommended seeds:

| Seed | Meaning |
|---|---|
| Neutral hue | Hue of the neutral UI surfaces |
| Neutral tint strength | How strongly neutral surfaces inherit the hue |
| Accent hue | Primary accent hue |
| Accent chroma | Accent colorfulness |
| Accent lightness nudge | Small adjustment to accent lightness |

Conceptually:

```text
5 seeds
   ↓
Light token ladder
   +
Dark token ladder
   ↓
Derived semantic tokens
   ↓
All UI components
```

Use modern perceptual color functions such as `oklch()` and `color-mix()` where supported by the production runtime.

### Why seeds are preferred

This keeps theme files small and makes themes consistent.

The author chooses:

```text
neutral personality
accent personality
```

rather than manually selecting dozens of unrelated colors.

### Optional override layer

A theme may define a small whitelist of token overrides.

Use overrides sparingly for cases such as:

- accessibility high contrast
- special neutral-mode needs
- a token that cannot be safely derived from the theme seeds

Overrides must never become a back door to arbitrary CSS.

---

## 47A.7 Fixed colors

Some colors should intentionally remain independent of the UI theme.

These include:

### Stripboard day/night convention

The production schedule's day/night strip colors should remain recognizable across themes.

### Printed call-sheet paper

Printed/exported call sheets should retain their document-paper appearance and must not become dark simply because the application is using Dark mode.

### Placeholder images

Placeholder image treatment should remain stable enough that missing/placeholder assets are recognizable.

### Semantic status colors

Information/success/warning/danger colors may use a fixed semantic hue family with mode-aware lightness rather than being radically recolored by every theme.

Theme changes must never make:

- warning look like success;
- error look like informational;
- selected state look like disabled state.

---

## 47A.8 Built-in theme set

The initial built-in collection should contain approximately 15 optional themes.

The names below are original OpenFrame names; do not market or ship themes under trademarked product, studio, film, character or brand names.

### 1. OpenFrame Default

**Purpose:** Canonical existing OpenFrame appearance.

This is the factory/default design.

It is not an optional “replacement look”; it is the baseline the entire application already uses.

### 2. Graphite

**Mood:** restrained graphite/steel.

**Best for:** focused writing, editing and users who prefer neutral interfaces.

### 3. Deep Navy

**Mood:** deep blue edit suite.

**Best for:** long evening writing and editing sessions.

### 4. Teal & Ember

**Mood:** cool surfaces with a warm cinematic action accent.

**Best for:** directors and filmmakers who want a cinematic personality without excessive saturation.

### 5. Parchment Room

**Mood:** warm paper/cream writing environment.

**Best for:** writers who prefer a softer reading surface.

### 6. Sage Cut

**Mood:** muted sage/green.

**Best for:** calm editing and planning sessions.

### 7. Burgundy Gold

**Mood:** theatre/premiere-inspired warmth.

**Best for:** presentation, development and creative work.

### 8. Sunset Coral

**Mood:** warm dusk/coral.

**Best for:** development and pitching.

### 9. Daylight Blue

**Mood:** crisp, clear blue-neutral interface.

**Best for:** schedules, production planning and structured desk work.

### 10. Violet Night

**Mood:** modern violet night interface.

**Best for:** distinctive creative sessions, especially users who prefer a more expressive appearance.

### 11. Olive Field

**Mood:** earthy olive.

**Best for:** grounded production/location planning.

### 12. Desert Sand

**Mood:** sand/rust earth tones.

**Best for:** users who prefer warm, natural surfaces.

### 13. Colorist Neutral

**Mood:** true neutral grey.

**Best for:** users who do not want the surrounding UI to introduce an obvious color cast.

### 14. Silver Screen

**Mood:** near-pure monochrome.

**Best for:** distraction-free writing and users who prefer maximum visual restraint.

### 15. High Contrast

**Mood:** accessibility-first.

**Best for:** users who need stronger text/surface separation and clearer boundaries.

### Required design constraint

These themes must be professionally balanced in both Light and Dark modes.

Do not simply invert the Light palette to create Dark mode.

---

## 47A.9 Theme picker UX

Settings should expose:

```text
Appearance

Mode
○ Light   ○ Dark   ● System

Theme
[ OpenFrame Default ]
  Canonical OpenFrame appearance

[ Graphite ]
  Restrained graphite / steel

[ Deep Navy ]
  Calm blue edit-suite

...
```

Each theme should have:

- name
- short description
- small live preview
- selected state
- light/dark preview appropriate to the current mode

### Apply behavior

Selecting a built-in theme:

- applies immediately or through a lightweight Apply button;
- changes only appearance;
- does not reload project data;
- does not modify screenplay formatting;
- does not affect exports unless the export itself intentionally uses UI branding/layout tokens.

---

## 47A.10 Theme import — untrusted external files

Users may download a theme from the OpenFrame website or another trusted source and import it without updating the application.

The application does **not** need an in-app theme server/API.

The website only needs to host downloadable theme files.

### File format

Use a small versioned JSON document.

Illustrative structure:

```json
{
  "formatVersion": 1,
  "id": "example-theme",
  "name": "Example Theme",
  "description": "A restrained example palette.",
  "author": "OpenFrame Community",
  "seeds": {
    "neutralHue": 220,
    "neutralTint": 0.018,
    "accentHue": 35,
    "accentChroma": 0.16,
    "accentLightnessNudge": 0.04
  },
  "overrides": {}
}
```

The actual production schema may use different property names, but it must remain versioned and strictly validated.

### Maximum size

Reject theme files above approximately **10 KB**.

Themes should remain small because they are data, not applications.

---

## 47A.11 Theme import flow

Path:

**Settings → Appearance → Import Theme**

Accept:

- file picker
- drag-and-drop
- paste theme code/JSON when that is exposed by the UI

Flow:

```text
Import
  ↓
Parse
  ↓
Validate schema
  ↓
Validate security
  ↓
Generate preview
  ↓
Contrast check
  ↓
[Apply] [Save to My Themes] [Cancel]
```

### Preview is mandatory before application for imported themes

The user should see:

- theme name
- author
- description
- Light preview
- Dark preview
- validation status
- contrast warnings if any

A failed import must change nothing.

---

## 47A.12 Theme validation — mandatory security boundary

Theme files come from outside the application and must be treated as untrusted input.

### Schema whitelist

Accept only known keys.

Reject unknown keys.

### ID validation

Theme ID must match:

```regex
^[a-z0-9-]{3,40}$
```

The ID must not collide with a built-in theme.

### Numeric range validation

Every seed must have an explicit permitted range.

Reject:

- NaN
- Infinity
- non-numeric strings
- values outside the documented ranges

### Override validation

If overrides are supported, only permit approved token names.

Override values may be only:

- `#hex`
- `oklch(...)`

Reject:

- `url()`
- `@import`
- `;`
- `{`
- `}`
- arbitrary CSS declarations
- HTML
- JavaScript
- CSS expressions/functions outside the whitelist

### Critical rule

The application must never inject raw theme-file text into the DOM.

Instead:

```text
Validated data
    ↓
internal token representation
    ↓
generated scoped style block
```

Only validated/generated values reach the style layer.

This prevents an imported theme from becoming an arbitrary code/CSS injection mechanism or impersonating the application UI.

---

## 47A.13 Contrast validation

Imported and built-in themes must be checked for readability.

At minimum validate:

- primary text vs primary background
- muted text vs its background
- accent text vs accent background
- relevant selected/hover states
- both Light and Dark modes

Target:

**WCAG AA 4.5:1 for normal text where applicable.**

When a theme fails:

1. warn the user;
2. auto-correct only when the correction is deterministic and does not materially change the theme;
3. otherwise reject the theme from application.

A visually attractive theme that makes writing difficult is not an acceptable OpenFrame theme.

High Contrast may intentionally use explicit token overrides to reach stronger separation.

---

## 47A.14 Generated style isolation

The runtime must generate one scoped theme style layer from validated internal values.

The theme system must not:

- append arbitrary `<style>` content from a file;
- evaluate theme data as code;
- execute scripts from theme files;
- import external assets;
- request remote resources.

A theme is **pure local data**.

---

## 47A.15 My Themes

Imported/saved themes live locally under:

**Settings → Appearance → My Themes**

Each saved theme can be:

- applied
- renamed
- exported
- deleted

### Active-theme deletion

If the active custom theme is deleted:

```text
Active Theme
     ↓
Delete
     ↓
OpenFrame Default
```

The fallback must happen immediately and safely.

Deleting a theme must never delete:

- projects
- scripts
- production data
- user files
- other themes

---

## 47A.16 Theme export

A user can export a saved theme as the same small JSON format.

Exported theme files should contain:

- schema version
- ID
- name
- description
- author
- seeds
- permitted overrides

Do not embed:

- executable code
- project data
- screenplay content
- user data
- network configuration

---

## 47A.17 Community/theme website model

The OpenFrame website may host downloadable `.json` theme files.

The app does not need:

- theme APIs
- live theme syncing
- account integration
- remote theme registry
- background network connection

The app simply imports a local file.

### Branding/copyright rule

Do not ship or officially host themes using:

- trademarked product names
- studio branding
- film franchise branding
- character branding
- copied proprietary visual identities

Use original theme names and original palettes.

For community submission, maintain a simple takedown/reporting policy.

---

## 47A.18 Color customization scope

Do **not** expose a complete freeform color editor in the first implementation.

The product should provide:

**Professionally designed presets first.**

A user may later get a small optional accent-color override if testing shows genuine value.

Avoid making users choose dozens of tokens such as:

```text
background hue
surface hue
sidebar hue
border hue
button hue
hover hue
...
```

That is exactly the complexity the theme system is intended to prevent.

---

## 47A.19 Theme settings persistence

Theme selection is an application/user preference, not project content.

Recommended behavior:

- store active theme locally
- store Light/Dark/System preference locally
- restore on next launch
- do not write theme preference into the screenplay/project database unless there is a later product reason to make themes project-portable

This keeps a user's appearance preference independent from film data.

---

## 47A.20 Do not theme the screenplay document itself

This distinction is mandatory.

### UI theme

Can change:

- editor chrome
- toolbar
- navigator
- panels
- buttons
- sidebars
- menus
- dialogs
- background surfaces

### Screenplay document

Must remain professionally formatted.

A dark UI user must still receive a conventional screenplay PDF.

The following must not change merely because a UI theme changes:

- screenplay paper color
- screenplay text color
- screenplay pagination
- screenplay margins
- screenplay font/preset
- title-page output
- revision semantics

---

## 47A.21 Theme-aware component design checklist

Every new UI component must answer:

1. Which semantic token does its background use?
2. Which semantic token does its text use?
3. Which border token does it use?
4. Which hover/selected/focus token does it use?
5. Which status token does it use?
6. Does it remain readable in Light and Dark?
7. Does it remain readable in High Contrast?
8. Does it accidentally rely on an old hard-coded color?

A component should not be accepted into production if it requires theme-specific one-off color hacks.

---

## 47A.22 Migration of the existing CSS design system

The uploaded prototype already demonstrates a tokenized approach in which `openframe.css` consumes semantic variables and `themes.css` defines the theme system. This should be used as an architectural reference while adapting to the real production stack.

The production task is therefore not:

> “Redesign every screen for 15 themes.”

It is:

> **“Complete tokenization of the existing OpenFrame design once, then let the theme engine supply the palette.”**

### Required migration audit

Search the entire UI codebase for:

- hex colors
- rgb()/rgba()
- named colors
- inline style colors
- SVG fill/stroke colors
- component-level color constants
- hard-coded CSS shadows where theme-aware shadows are required

Classify each result as:

**Theme token / intentional fixed / document output**

Then remove themeable literals.

---

## 47A.23 Theme implementation simplicity requirement

The following must be a hard acceptance criterion:

> **Adding a new built-in theme should require one seed declaration plus theme metadata, not edits to individual UI components.**

For example, conceptually:

```css
[data-theme="heroic-red-blue"] {
  --h: 230;
  --c: .02;
  --ah: 8;
  --ac: .19;
  --ak: 4%;
}
```

and one registry entry:

```text
heroic-red-blue
Heroic Ember
Warm red/blue cinematic contrast
```

No other component should need modification.

The actual production implementation can use typed TypeScript/config objects instead of CSS declarations, provided it preserves this one-definition principle.

---

## 47A.24 Testing requirements

### Theme rendering

For every built-in theme:

- Light works
- Dark works
- System works
- switching is immediate
- application remains usable
- no component visually breaks

### Import safety

Test:

- valid theme
- unknown key
- invalid ID
- duplicate built-in ID
- duplicate custom ID
- oversized file
- invalid JSON
- non-numeric seeds
- out-of-range seeds
- malicious CSS function
- `url()`
- `@import`
- `;`
- braces
- malformed overrides
- contrast failure

Invalid theme:

```text
No data changes.
No theme changes.
No project changes.
No network activity.
```

### Persistence

Test:

- restart application
- selected theme restored
- mode restored
- invalid stored theme falls back safely
- deleted active custom theme falls back safely

### Accessibility

Run contrast tests for:

- primary text
- muted text
- accent text
- buttons
- selected states
- warning/error/success states

in:

- Light
- Dark
- High Contrast

### Regression

Every existing OpenFrame screen should remain structurally identical when using **OpenFrame Default**.

This is critical: introducing theming must not become a hidden UI redesign.

---

## 47A.25 Acceptance criteria

The theme system is complete only when all are true:

- The existing OpenFrame appearance remains the factory default.
- Existing users do not unexpectedly receive a new visual design.
- Light, Dark and System modes work.
- System follows OS preference and updates live.
- All themeable UI colors use semantic tokens.
- No individual component contains a theme-specific hard-coded palette.
- Built-in themes are centrally defined.
- Approximately 15 professional built-in themes are available.
- A new theme can be added without editing components.
- Users can import a small local JSON theme file.
- Imported themes preview before application.
- Users can save imported themes to My Themes.
- Saved themes can be renamed/exported/deleted.
- Active-theme deletion falls back to the default.
- Theme files are strictly schema-validated.
- Unknown keys are rejected.
- Theme IDs follow `^[a-z0-9-]{3,40}$`.
- Numeric seed ranges are enforced.
- Theme files are capped at about 10 KB.
- Unknown major format versions are rejected.
- Arbitrary CSS cannot be injected.
- Theme imports cannot load remote URLs or execute scripts.
- Contrast is checked in both Light and Dark modes.
- WCAG AA 4.5:1 is the target for applicable normal text.
- Invalid/malicious files change nothing.
- Theme preferences remain local and offline.
- No in-app theme server/API is required.
- Screenplay output remains professional and independent of UI theme.
- Fixed production/export colors remain intentionally stable.
- Current OpenFrame Default rendering passes a visual regression comparison against the pre-theme baseline.

---

## 47A.26 Recommended initial built-in registry

The release registry should expose the canonical default plus the optional themes:

```text
openframe-default  — OpenFrame Default
graphite           — Graphite
deep-navy          — Deep Navy
teal-ember         — Teal & Ember
parchment-room     — Parchment Room
sage-cut           — Sage Cut
burgundy-gold      — Burgundy Gold
sunset-coral       — Sunset Coral
daylight-blue      — Daylight Blue
violet-night       — Violet Night
olive-field        — Olive Field
desert-sand        — Desert Sand
colorist-neutral   — Colorist Neutral
silver-screen      — Silver Screen
high-contrast      — High Contrast
```

### Note on the existing prototype

The uploaded prototype currently demonstrates 15 seeded theme entries and Light/Dark/System switching. Its default seed profile is labeled `Studio Amber`, while the product requirement here is that the **existing OpenFrame appearance remains the canonical default**.

Therefore, during production integration:

- preserve the current visual design exactly;
- map its existing palette to `openframe-default` (or retain its internal seed name if compatibility makes that safer);
- treat all other profiles as optional additions;
- do not make the presence of the theme engine itself a reason to recolor the existing product.

---

## 47A.27 Product-level outcome

The theme system should make OpenFrame feel personal without making it feel configurable in the way a design tool is configurable.

The user experience should be:

```text
Open OpenFrame
   ↓
The familiar OpenFrame design
   ↓
Write normally
```

and, only when desired:

```text
Settings
   ↓
Appearance
   ↓
Choose a professional theme
   ↓
Continue working
```

The product remains the same OpenFrame.

Only its visual atmosphere changes.
