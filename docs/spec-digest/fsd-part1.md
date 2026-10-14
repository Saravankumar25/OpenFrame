# FSD Digest — Part 1 (Sections 1–52)

Source: `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md`, lines 1–2130.
Companion specs referenced by the FSD: `OpenFrame_Studio_Mega_PRD_Aligned_Updated.md` (source PRD), `OpenFrame_Studio_AI_Specification_Updated.md` (cross-cutting AI).

## Legend

- **[REQ]** — hard requirement: spec says must / required / never / exactly / "is required".
- **[SHOULD]** — spec says should; implement unless there is a documented reason not to.
- **[MAY]** — spec permits but does not mandate (may / optional / "if useful" / "when available").
- **[LATER]** — spec explicitly defers or conditions it ("if supported", "where supported", "documented separately", "not in the initial product").
- **[DECISION]** — marked "FSD Decision" in source (product decision, binding).
- **ID Pn** — requirement IDs do NOT appear inline in sections 1–52. IDs shown here are cross-referenced from the FSD acceptance matrices §68–§70 (lines 2559–2610) and are quoted verbatim with their inherited priority. P0 = v1-required core; P1 = v1-required secondary (per PRD priority).
- `Lnnn` = line number in the source FSD.
- Quoted strings in "…" are exact UI labels/strings from the spec.

Global rules on priority (L39–49): all P0/P1/P2 inherit from the PRD; FSD cannot promote/demote/create scope. Each FSD requirement is supposed to carry: PRD Requirement ID, FSD Requirement ID, inherited priority, functional behavior, acceptance criteria (template only — sections 1–52 do not populate it).

Product scope (L17): project creation, Global/Project Idea Vaults, Project Home, Story Board, Acts, Sequences, Beats, Scene Cards, Characters, Story Timeline, Screenplay Workspace, screenplay import/export, draft/version history, comparison, review rounds, comments, script lock + post-lock revisions, episodic, breakdown, auto-breakdown suggestions w/ confirmation, production catalog, locations, cast/crew, moodboards, storyboards, shot lists, stripboard/scheduling, scheduling assistance, call sheets, project files, global search, budget snapshot, AI assistant, offline, exchange packages, LAN collaboration, permissions, undo/redo, deletion/recovery, saving, portability, backups, activity history, reports, sides, daily production view, templates, exports, printing, empty states, errors, source-of-truth rules.

**Explicit exclusions (L19) [REQ — must NOT build]:** enterprise ERP, payroll, union accounting, full accounting, advanced studio resource planning, VFX production tracking, post-production asset management, distribution CRM, festival submission suite, legal-management suite, talent marketplace, equipment marketplace, mandatory cloud-sync platform.

Platform/operating model (L7–9): Windows + macOS desktop; local-first, offline-capable, user-owned project files; optional internet AI; optional LAN collaboration; no mandatory OpenFrame cloud.

---

## §1 Functional Contract and Non-Negotiable Rules (L51–86)

Violating any of these = functionally incorrect even if the UI works.

- **1.1 One app, many workspaces [REQ]:** single desktop app; moving between Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets must not create duplicate projects or require manual copy of ordinary metadata.
- **1.2 Progressive structure [REQ]:** Idea Vault (least structured) → Story Board (light visual structure) → Screenplay (strict writing structure) → Breakdown/scheduling (production structure). Progression must be visible to the user.
- **1.3 No duplicate entry [REQ]:** if info exists upstream, reuse it or offer import/conversion. Must NOT require retyping scene heading/title just to move forward. Duplicate entry allowed only as deliberate copy.
- **1.4 No hidden sync [REQ]:**
  - Idea Vault is NOT a live-synced DB of screenplay content.
  - Story Board is a reference/outline surface, NOT an always-synced copy of the script.
  - Production data is derived from a selected screenplay/source version; production changes must NOT silently rewrite the screenplay.
- **1.5 Safe experimentation [REQ]:** duplicate cards, create drafts, move cards to Parking Lot, undo — without destroying earlier work.
- **1.6 User control over automation [REQ]:** AI/automatic parsing only suggests; user confirms anything that becomes meaningful project data. No AI operation may silently change story, script, breakdown, schedule, or call sheet info.
- **1.7 Offline ownership [REQ]:** lack of internet cannot prevent open/edit/save/export of a local project.
- **1.8 File exchange over cloud [REQ]:** remote collaboration defaults to portable exchange packages; real-time collab = optional LAN session; no OpenFrame cloud required for ordinary use.
- **1.9 Short films first-class [REQ]:** short film may use only Story Board + Screenplay + a few production features; must NOT force empty enterprise sections.
- **1.10** Professional output, short/approachable input forms.
- **1.11 Manual overrides automatic [REQ]:** when an automatic suggestion conflicts with a manual decision, manual decision is authoritative.

## §2 Terminology and Object Identity (L89–115)

Canonical objects and authority:

| Object | Meaning | Authoritative for |
|---|---|---|
| Global Idea Vault Item | loose creative item, filmmaker-owned, cross-project | the item itself |
| Project Idea Vault Item | project-scoped creative item | the item itself |
| Act | high-level Story Board container | Story Board structure |
| Sequence | named container of story scenes/cards | Story Board grouping |
| Beat | small story event/idea | the Beat object |
| Scene Card | compact outline/reference of a scene | the card — NOT screenplay text |
| Screenplay Scene | actual scene in screenplay | written script content |
| Screenplay Draft | named, user-recognized screenplay version | that draft |
| Automatic History Point | recoverable background version | restore source only; NOT a deliverable draft |
| Breakdown Element | production need associated to a screenplay scene | production planning |
| Catalog Item | reusable production item (prop/location/person) | catalog identity |
| Shot | planned camera/coverage unit | shot planning |
| Shooting Day | scheduled production day | schedule |
| Call Sheet | document prepared for a shooting day | that document; NOT automatically authoritative over schedule |
| Exchange Package | portable package for review/import/merge | snapshot of source at export time |

- **[DECISION] Internal identity vs display numbering (L111):** scene cards and story objects retain stable internal identities even when IDs/scene numbers are not shown. Display scene numbers are generated for screenplay and production documents from ordered screenplay scenes; NEVER manually maintained on Story Board cards.
- **2.1 Identity rule [REQ]:**
  - Move → same object (no new identity).
  - Duplicate → new object (new identity).
  - Import exchange package → create/update per package rules; must NEVER treat a copy as same identity unless the package explicitly represents a review/change to an existing object.

## §3 Application Shell and Global Navigation (L117–177)

- **3.1 Main shell:** shell stable while central workspace changes; window resize adapts active workspace without spawning unrelated popups.
  - Default left navigation, in order: "Home", "Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets", "Files".
  - Top-level controls: current project selector; global search; save/sync/session indicator; Undo/Redo; Help/command access; user/project access controls.
- **3.2 [REQ]** Open project's title visible in persistent shell; on project change, shell title updates immediately after new project becomes active.
- **3.3 Save state indicator [REQ]** — must distinguish at minimum:
  - "Saved"
  - "Saving"
  - "Saved with pending external/package operation"
  - "Save error"
  - Core editing continues during transient save work.
  - On save failure: visible, non-blocking explanation + clear retry/recovery path.
- **3.4 Project switching [REQ]:** with unsaved work, either complete save or ask user: stay / save / cancel switch. Never silently abandon changes.
- **3.5 Command labels (consistent labels + placement):** "New", "Open", "Duplicate", "Rename", "Delete", "Undo", "Redo", "Export", "Import", "Search", "Settings". Context-specific actions may appear in context menus.
- **3.6 Context menus [SHOULD]:** right-click shows only actions meaningful for that object (e.g., Scene Card must not show payroll or call-sheet commands).

## §4 Project Creation and Lifecycle (L179–225)

- **Entity: Project.** Fields: Title, Project type, Language, Genre, Creator, status, local identity, last opened (see §6), archived flag.
- **4.1 New Project** opens minimal creation dialog.
  - Required: **Title**; **Project type** ∈ {"Feature Film", "Short Film", "Episodic", "Series"}.
  - Optional: Language, Genre, Creator.
  - [DECISION] NO budget, schedule, team, cast, or location fields required at creation.
- **4.2 Creation result [REQ]:** on confirm → project created and opened at **Project Home**; contains empty workspaces immediately; no setup wizard required.
- **4.3 Project status** labels: "Idea", "Development", "Writing", "Rewrite", "Shooting Draft", "Pre-Production", "Shoot Preparation", "Shooting", "Archived".
  - Manually changeable.
  - [MAY] app suggests status from activity; [REQ] must NOT auto-change user's chosen status.
  - Default status at creation: not specified.
- **4.4 Duplicate Project:** creates separate project package with copied content; new project gets its own local identity [REQ]; UI shows "Copy of …" until renamed [SHOULD].
- **4.5 Archive:** hides from normal recent-project views; not deleted; openable via an "Archived/All Projects" view.
- **4.6 Restore (from archive):** returns to active projects; contents unaltered.
- **4.7 Close project:** returns to application home/recent projects view; does not delete.

## §5 Idea Vault — Global and Project (L228–292)

ID refs: **FSD-IDEA-001 P0** (create text item without metadata), **FSD-IDEA-002 P0** (add image, URL, PDF, document, audio, video, screenshot, sketch, ordinary file), **FSD-IDEA-003 P0** (Global and Project vaults independent), **FSD-IDEA-004 P0** (Move to Story copies; original intact).

- **5.1** Unstructured by design. Primary action label: **"Add"** (not "classify"). Interpretation can be postponed.
- **5.2 Supported item types [REQ, "at least"]:** text notes; images; URLs; PDFs; office/document files; audio files + recorded voice notes; video files; folders/collections; handwritten/sketch images; quotes; screenshots; ordinary attached files supported by host OS.
- **Entity: VaultItem** — fields mentioned: type, title (optional), note text, caption, filename, tags, pins, links to project objects, folder, collections, URL, attachment/file, transcription (voice), generated display name, scope (global|project), deleted state.
- **5.3 Add flow:** user selects Add → chooses type, OR drags supported content into Vault. Request only minimum metadata needed to store.
  - Text → open editor immediately.
  - Image/file → ingest/display; allow optional title/note.
  - URL → save URL + user note; [MAY] metadata preview when available (online-only, see §43.2).
  - Voice note → record, stop, save; transcription optional [MAY].
- **5.4 Untitled items valid [REQ].** App may generate a local display name for navigation; generated name is NOT treated as user-created content (i.e., not stored as title / not searchable-as-authored — distinguish flag).
- **5.5 Views:** "Visual Grid", "Card View", "List View", "Folder View". Presentation only; no data duplication.
- **5.6 Folders/Collections:** folders are user-selected containers; collections = thematic groups. An item may be in one folder AND associated with tags/collections without physical duplication.
- **5.7 [REQ]** Tags, pins, links to project objects, captions, notes are optional; Add flow must never require them.
- **5.8 Global vs Project:** Global Vault available outside any project; Project Vault scoped to current project.
  - Copy Global → Project creates project-local copy (new identity). Editing the copy does NOT modify global original.
- **5.9 Vault search:** matches titles, note text, captions, filenames, tags. No semantic classification required.
- **5.10 "Move to Story" (actually a copy):** Vault item can be copied into Story as Beat, Scene Card, Sequence idea, Character note, or generic Story note.
  - Original remains in Vault [REQ].
  - Story object [MAY] show informational "Source: Idea Vault" reference.
  - No live sync established [REQ].
- **5.11 Delete:** Vault item → recoverable deleted items first; permanent deletion requires deliberate confirmation.
- **AC:** Given a text note with no metadata, When saved, Then item persists (IDEA-001). Given a global item copied to project, When project copy edited, Then global original unchanged (IDEA-003). Given Move to Story, When destination edited, Then Vault original unchanged (IDEA-004).

## §6 Project Home and Continue (L294–337)

- **6.1 Required areas:**
  - Project header: title, type, status, last opened.
  - "Continue": contextual shortcuts from latest activity.
  - Quick Access: Idea Vault, Story Board, Screenplay, Breakdown, Production, Call Sheets.
  - Recent: recently changed scenes/cards/drafts/documents.
  - Project Files: lightweight file cabinet.
  - Must remain visually simple.
- **6.2 Continue:** app remembers last meaningful location per project; "Continue" returns there when possible.
  - Last open Scene Card → Story Board with that card opened.
  - Last screenplay scene → screenplay at that location.
  - Last breakdown scene → breakdown on that scene.
  - Last shooting day → schedule opened to that day.
- **6.3 Empty project [REQ]:** no fake metrics. Shows exactly three actions: 1. "Open Idea Vault" 2. "Build Story" 3. "Write/Import Screenplay".
- **6.4 Production-stage home [MAY]:** once production content exists, may also show: script status; breakdown progress; next shooting day; outstanding location/cast needs; latest call sheet; recent production document. These are shortcuts, NOT a mandatory analytics dashboard.

## §7 Story Workspace and Story Board (L340–374)

- **7.1** Story Board = visual reference/outlining surface; not a screenplay editor; not a DB form.
- **7.2 Views:** "Board View" (card-based visual) and "Outline View" (compact hierarchical text/list). Same underlying structure.
- **7.3 Hierarchy (default):** Act → Sequence → {Beat (optional), Scene Card…}.
  - Scene Card may exist directly under an Act (no Sequence).
  - Beat may exist independently in Story Board/Parking Lot before promotion to Scene Card.
  - Parent types: Act children = Sequence | Scene Card | Beat; Sequence children = Scene Card | Beat; Parking Lot holds Beat | Scene Card. (Note: an "Unassigned" area is referenced in §8.5 and §52.5 but not defined in the hierarchy.)
- **7.4 Board layout:** Acts visually separated; Sequences as groups within Acts; Scene Cards as compact rectangles; horizontal scroll or zoom for large boards.
- **7.5 Outline View:** shows indentation + order; for precise hierarchy management, not extra metadata entry.
- **7.6 [REQ]** Switching Board ↔ Outline must not alter order or content.

## §8 Acts (L377–398)

ID ref: **FSD-STORY-001 P0** (Acts contain sequences and/or scenes).

- **Entity: Act** — title, optional note, order, collapsed state, children.
- **8.1 Create:** "Add Act" → new Act appended at END of Story Board; user enters title; optional note available after creation.
- **8.2** Contains Sequences and/or Scene Cards; collapsible/expandable.
- **8.3 Rename:** double-click title OR context menu → "Rename". Updates everywhere Act is referenced in Story Board displays/exports; does NOT alter screenplay text.
- **8.4 Reorder:** drag Act; all child sequences/scenes move with it.
- **8.5 Delete:** confirmation required if it has children. Choices:
  - delete Act and children;
  - move children to another Act / Unassigned;
  - cancel.
  - [DECISION] default confirmation recommends moving children out (not deleting creative material).
  - (Empty Act: no confirmation required per text; still soft-delete per §52.1.)

## §9 Sequences (L401–432)

ID ref: **FSD-STORY-002 P0** (simple named scene container; one name field).

- **Entity: Sequence** — name (single text field), parent Act, order, children. No required "purpose" field [REQ — must NOT exist as required].
- **9.1** Named container for a group of scenes; NOT a formal production object.
- **9.2 Create:** "Add Sequence" under an Act; primary field is text. Example names: "Railway Station Scenes", "Investigation", "Hero Introduction", "Chase", "Family Confrontation".
- **9.3 Children:** normally Scene Cards; Beats may temporarily exist inside.
- **9.4 Reorder:** drag within Act; contained cards move with it.
- **9.5 Move between Acts:** drag to another Act re-parents; scene identities unchanged [REQ].
- **9.6 Delete with children:** safe choice — (a) delete container only after moving children elsewhere, or (b) delete all with explicit confirmation.
- **9.7 [REQ]** Sequence does NOT automatically become a screenplay sequence; name is organizational aid unless user includes hierarchy in an outline export.

## §10 Beat Cards (L435–459)

- **Entity: Beat** — beat text (the one required text field), optional note, optional color, optional attachment, parent (Act | Sequence | Parking Lot), order, state (active | "Converted").
- **10.3 Create:** in a Sequence, Act, or Parking Lot.
- **10.4 Drag:** Beat can move anywhere board accepts a Beat; changes order and parent; NO version branch created.
- **10.5 "Convert to Scene":** creates new Scene Card with Beat text as initial short description.
  - Original Beat default: retained in a "Converted" reference state (no creative thought lost). Spec also allows "history/reference object or marked converted according to UI choice".
- **10.6** Beat can stay in Parking Lot indefinitely with no screenplay impact.

## §11 Scene Cards — Exact Behavior (L462–529)

ID refs: **FSD-STORY-003 P0** (compact, short description, no manual numbering — verify by creating/reordering 20 cards), **FSD-STORY-004 P0** (drag changes order and is undoable).

- **Entity: SceneCard** — short description, optional scene heading (e.g., `INT. POLICE STATION — NIGHT`), freeform scene notes, attachments, comments, parent (Act | Sequence | Parking Lot), order, deleted state, optional screenplay relationship ("used to create screenplay" reference), optional character links (§13.4), optional "Source: Idea Vault" reference (§5.10).
- **11.2 Required at creation:** short description (may be blank while entering a new card). Heading optional.
- **MUST NOT be Story Board card fields [REQ]:** scene number; characters; story day; production day; props; wardrobe; breakdown categories; budget; shot list. (Derived/managed downstream.)
- **11.3 Collapsed display:** short description; optional small indicator that a header exists; small status indicators only when useful (e.g., comments count) — never a metadata wall. Long descriptions truncate visually; stored text never truncated [REQ].
- **11.4 Expanded card/drawer (double-click):** optional scene heading; short description; freeform scene notes; attachments; comments.
- **11.5 Create:** inserted at drop position OR end of selected sequence; description immediately editable.
- **11.6 Reorder:** drag-drop changes order instantly; move recorded for undo [REQ].
- **11.7 Duplicate:** new Scene Card copying description, optional heading, notes, and *selected* attachments. New identity; does NOT inherit screenplay scene identity/relationship [REQ].
- **11.8 Delete:** → recoverable deleted state. If card has a screenplay relationship → warning that linked screenplay scene is NOT automatically deleted [REQ].
- **11.9 Parking Lot:** drag card into Parking Lot → removed from active outline, recoverable.
- **11.10 No live sync [REQ]:** editing card after screenplay creation does not rewrite screenplay scene. [MAY] show simple "used to create screenplay" reference.
- **11.11 No permanent scene number [REQ]:** numbers computed from screenplay scene order; card reorder needs no renumbering.
- **11.12 Heading at conversion [REQ]:** Build Screenplay must identify included cards lacking a valid screenplay heading and request the user supply it before creating a normal screenplay scene; user may alternatively exclude those cards.
- **AC:** Given 20 cards, When reordered, Then no numbers shown/edited and order persists (STORY-003). Given a drag, When Undo, Then prior order restored (STORY-004). Given a card linked to a screenplay scene, When deleted, Then warning shown and screenplay scene remains.

## §12 Parking Lot and Experimentation (L531–553)

ID ref: **FSD-STORY-005 P0** (Parking Lot preserves unused cards — move, close/reopen project, restore).

- Simple holding area; NOT a branch-management engine.
- **12.1** Any Beat or Scene Card can be moved to Parking Lot.
- **12.2 Restore:** drag back into Act/Sequence OR "Restore to Story".
- **12.3 Experimentation mechanisms (only these):** duplicate Beat/Scene Card; reorder; move to Parking Lot; make a named screenplay draft.
  - [LATER / excluded from initial product] multi-branch story graph, branching state tree, alternate-ending comparison subsystem.
- **12.4 [SHOULD]** Parking Lot cards visually distinct (not part of active story order).
- **AC:** Given card in Parking Lot, When project closed and reopened, Then card still in Parking Lot and restorable (STORY-005).

## §13 Characters (L555–580)

ID ref: **FSD-CAST-001 P1** is cast-side (see §30).

- **Entity: Character** — Name (required); optional short description, role label, image, notes; archived/removed-from-directory state; scene appearances (derived from screenplay parsing, user-correctable); optional Story Board card links.
- **13.1** Simple project directory for story reference and later production linkage.
- **13.3 References:** character page shows screenplay scenes where the character appears once screenplay parsing identifies them. User can correct false detections (override persists — §1.11).
- **13.4 Manual link [MAY]:** associate character with a Story Board card for reference; never required for card to exist.
- **13.5 Rename [REQ]:** must NOT blindly change arbitrary text in screenplay dialogue/action (screenplay has its own replace controls). Production identity references may be updated as relationships change.
- **13.6 Delete:** if used in screenplay/production → confirmation required; offer archive / remove-from-directory rather than destructive removal by default.

## §14 Story Timeline (L582–599) — optional feature

- **Entity:** Story Day assignment on screenplay scene; optional time-of-day notes.
- **14.1** Simplest unit = Story Day (+ optional time-of-day notes).
- **14.2** Assign story days to screenplay scenes after scenes exist. Story Board cards do NOT require Story Day (and per §11.2 must not store it).
- **14.3 Display:** scenes ordered by story day; unassigned scenes grouped under "Unassigned".
- **14.4 [MAY]** soft continuity warning if a later event is assigned to an earlier day than a referenced dependent event. [REQ] never auto-reorder scenes.
- **14.5 Episodic:** story-day scope is episode-scoped or series-scoped per project setting; **default = episode-scoped**.

## §15 Screenplay Workspace — Overall (L602–643)

ID ref: **FSD-SCRIPT-001 P0** (required element types).

- **15.1** Authoritative written document area.
- **15.2 Entry paths:** "Create Screenplay"; "Open current screenplay"; "Open a draft"; "Import screenplay"; "Build from Story Board".
- **15.3 Modes:**
  - "Focus Mode" — page dominates window.
  - "Standard Mode" — scene navigator + compact tools visible.
  - "Writing Room Mode" — screenplay + optional Story Board / scene notes / character / comments panel (see §17).
- **15.4 Authority [REQ]:** screenplay owns written scene content. Breakdown uses a chosen screenplay/source version. Story Board stays reference unless the user explicitly uses a board action to create/reorder screenplay content.
- **15.5 Element types [REQ]:** scene heading; action; character; dialogue; parenthetical; transition; optional shot direction; user notes outside printed script.
- **15.6 Auto-formatting [SHOULD]:** apply screenplay formatting when moving between common element types; writer must be able to override element type [REQ].
- **15.7 Scene navigator:** lists scenes with generated scene numbers + headings; click jumps to scene in same document (no new document).
- **15.8 In-script search:** find, next, previous, replace, case sensitivity, whole-word. No semantic story search required.

## §16 Screenplay Writing Interactions (L646–664)

- **16.1 [REQ]** Cursor movement, selection, copy/paste, undo/redo, common text shortcuts behave like a professional writing app; no custom interaction model.
- **16.2** Element type changeable via compact selector AND keyboard shortcut. Contextual auto-selection may suggest next type; must remain overridable.
- **16.3 Scene creation:** from scene navigator or within editor. Display number derived from position; user never types the number [REQ].
- **16.4 Pagination:** app maintains standard screenplay page layout for PDF/print; writer does not insert page breaks manually for normal text.
- **16.5 Title page fields:** project title, author/creator, contact fields, optional revision/draft info (per export configuration).
- **16.6 Notes:** scene notes outside printed screenplay; show/hide toggle while writing; excluded from normal PDF export unless user explicitly chooses a notes export [REQ].

## §17 Writing Room Layout (L667–696)

- Layout only; NOT a separate data model.
- **17.1 Default:** Left = scene navigator; Center = screenplay; Right = optional panel.
- **17.2 Right panel options:** "Story Board reference", "Scene Notes", "Characters", "Comments", "Draft information". Normally only 1–2 visible at once.
- **17.3 [REQ]** No permanent Idea Vault sidebar; Vault accessed via normal nav + search; not a synced writing-room panel.
- **17.4 Persistence:** remember last layout per project/user; opening a script must not reset preferred panel arrangement.
- **17.5 Panel independence [REQ]:** opening/viewing a panel never edits underlying content (viewing a Story Board card does not lock or modify screenplay).

## §18 Build Screenplay From Story Board (L699–732)

ID ref: **FSD-SCRIPT-002 P0** (build creates scenes from selected cards — verify order/headings).

- **18.1 Entry:** Story Board toolbar → "Build Screenplay".
- **18.2 Preconditions:** select all active scene cards or a subset. Cards without headings must be resolved (heading supplied) or excluded before conversion [REQ].
- **18.3 Preview (before creation):** ordered table with columns exactly: "Order" | "Story Board scene" | "Heading" | "Include". User can deselect cards.
- **18.4 Build action (on confirm):**
  1. Create a screenplay draft from selected cards.
  2. Create screenplay scenes in the same order as cards.
  3. Carry over the heading.
  4. Place the card description as a planning note OR opening action placeholder "according to the chosen creation mode" (mode options not further defined).
  5. Do NOT copy unrelated Idea Vault metadata.
  - Card gets "used to create screenplay" reference (§11.10); scene numbers derived afterwards.
- **18.5 Repeat build [REQ]:** must not silently overwrite existing screenplay. Options: "create new screenplay draft"; "build new screenplay document"; "cancel".
- **18.6** Board changes are not auto-applied to script. User may intentionally reorder/rebuild; app must warn if operation could affect written screenplay order [REQ].
- **AC:** Given cards A,B,C (B missing heading), When Build Screenplay, Then B flagged; user supplies heading or excludes; resulting scenes follow card order with carried headings.

## §19 Screenplay Import (L735–784)

- **19.1 Sources [REQ]:** PDF; Final Draft FDX; Fountain; TXT; DOCX; pasted screenplay text.
- **19.2 Modes:** "New screenplay" | "New draft in current screenplay project". Never overwrite existing draft by default [REQ].
- **19.3 Pre-import preview [SHOULD]:** detected scenes; character names; approximate page count; document title; obvious parsing warnings.
- **19.4 PDF:** attempt structure recognition; if confidence insufficient → show review state; allow continuing with editable text; must not present parse as perfect.
- **19.5 DOCX:** extract readable text + formatting where possible → map to elements; unsupported formatting NOT silently treated as screenplay structure.
- **19.6 TXT/paste:** parsing preview; user corrects scene headings / element interpretation before confirming.
- **19.7 FDX:** preserve content + supported draft/revision metadata; unsupported properties ignored gracefully, reported in import summary if relevant.
- **19.8 Import report (post-import):** imported scenes; detected characters; pages/length; warnings requiring attention; source filename.
- **19.9 Source preservation [MAY, user choice]:** original file can be retained in Project Files. Imported screenplay is the editable OpenFrame object.
- Suggestions may also be offered after import for breakdown (§27.1).

## §20 Screenplay Export (L787–819)

- **20.1 Required formats [REQ]:** PDF, FDX, Fountain, DOCX.
- **20.2 Scope:** entire current draft; selected scenes [LATER: "where supported"]; clean script; revision-marked version when applicable.
- **20.3 PDF:** professionally formatted, printable; notes excluded unless explicitly selected.
- **20.4 FDX:** preserve structure/content that maps to FDX; unsupported OpenFrame metadata retained internally; must not block export.
- **20.5 Fountain:** standard Fountain conventions; internal metadata NOT inserted unless user selects optional header/comment export.
- **20.6 DOCX:** editable document; readable screenplay layout.
- **20.7 Completion:** show file location + offer "Reveal in File Manager".
- **20.8 Immutability:** exports are snapshots; editing exported file externally does not change project.
- Comments: normal screenplay PDF excludes internal comments (§23.8).

## §21 Drafts, Automatic History, Version Control (L822–854)

ID ref: **FSD-SCRIPT-004 P0** (named drafts preserve prior versions — create multiple, compare).

- **Entities:** ScreenplayDraft — name, optional note, parent draft (lineage), isCurrent, deleted state, locked flag (§24), text content. AutomaticHistoryPoint — restore/crash-recovery only.
- **21.1 Two layers:** (1) automatic recoverable edit history; (2) named drafts created intentionally.
- **21.2** Automatic history = restoration + crash recovery only; never treated as deliverable version.
- **21.3 "New Draft":** copies selected source draft into new version; user provides name + optional note.
- **21.4 Lineage:** each draft records parent; UI shows simple lineage, e.g. `Draft 1 → Draft 2 → Director Rewrite → Shooting Draft`.
- **21.5 [REQ]** Exactly one draft marked Current in the screenplay workspace at any time.
- **21.6** Export clean distribution copy from any named draft; export does NOT change Current.
- **21.7 Restore old draft:** creates NEW draft "restored from Draft 2" (label pattern) rather than replacing current.
- **21.8 Rename:** changes only label, never text.
- **21.9 Delete:** confirmation required; → recoverable deleted state. Current draft cannot be permanently deleted without first selecting another Current draft.

## §22 Draft Comparison (L857–884)

- **22.1 Entry:** Draft History → select two named drafts → "Compare".
- **22.2 Two layers:** change summary by scene; exact text comparison within a selected scene.
- **22.3 Scene-level summary categories:** added; removed; moved; changed text; unchanged.
- **22.4 Text diff:** side-by-side or inline markers; added vs removed visually distinguishable [REQ].
- **22.5 [REQ]** Read-only; no automatic merge. User edits manually or creates new draft.
- **22.6 Matching:** match by screenplay scene identity where available; fallback heading/order heuristics for imported legacy docs; ambiguous matches flagged, never silently assumed [REQ].

## §23 Review Rounds and Comments (L887–923)

ID ref: **FSD-SCRIPT-005 P0** (comments created and resolved — test: add comment, reply, resolve).

- **Entity: ReviewRound** — source draft, reviewers, date, optional deadline, status (values beyond "Complete" not enumerated).
- **Entity: Comment** — target (see below), text-range anchor (optional), status, "context moved" flag, nearest-scene fallback link.
- **23.1 Create:** select draft → "Start Review".
- **23.2 Comment targets:** screenplay text; scene card; beat; sequence; act; storyboard panel; shot; location; breakdown element.
- **23.3 Lifecycle:** `Open → In Discussion → Resolved`. Resolved remain accessible in review history.
- **23.4 Anchoring:** screenplay comments stay attached to intended text region where possible; if text no longer exists → status/flag "context moved" and link to nearest surviving scene; never disappear [REQ].
- **23.5 General scene comment:** no text range; attached to scene object.
- **23.6 Permissions:** Writer/Director/Producer/Reviewer may comment per project role + sharing config. Viewer cannot comment unless explicitly granted Commenter capability.
- **23.7 Complete review:** only users with appropriate permissions; completing does NOT auto-lock script [REQ].
- **23.8 Export:** review packages may include comments; normal screenplay PDF excludes internal comments.

## §24 Script Lock and Production Revisions (L926–966)

ID ref: **FSD-SCRIPT-006 P0** (locked script cannot be casually edited without starting a revision).

- **24.1 Lock:** Draft History → select named draft → "Lock as Shooting Draft".
- **24.2 Confirmation dialog contents:** draft name; current draft status; open review comments (count/list); last modified date; scene count. Must explicitly state that future edits will create a post-lock revision flow [REQ].
- **24.3 Locked state:** readable + exportable; editing only via "Start Revision".
- **24.4 "Start Revision":** creates new revision based on locked source; locked source unchanged [REQ].
- **24.5 Revision labels (examples, production-chosen):** "Revision A", "Revision B", "Blue Revision", "Pink Revision", "Yellow Revision". Naming free-form.
- **24.6 Revision color:** applies to revision metadata + supported export presentation; user can choose custom color.
- **Entity: Revision** — label, color, locked-source reference, changed-scene list, approval status ("approved revision" referenced but approval flow not defined here).
- **24.7** Revision view lists affected scenes; user can inspect exact changes.
- **24.8 Production source:** breakdown may select locked shooting draft or later approved revision; chosen source recorded and visible [REQ].
- **AC:** Given locked draft, When user attempts to type, Then edit blocked and "Start Revision" offered; source text unchanged after revision edits.

## §25 Episodic Projects (L969–995)

- **25.1 Hierarchy:** Series project → Seasons → Episodes.
- **Entity: Episode** — episode number/order, title, optional one-line summary, status (values not enumerated), parent Season.
- **25.3** Each episode has its own Story Board, screenplay drafts, breakdown, production planning.
- **25.4** Series-level lightweight directories: recurring characters and locations; episode records can reference them.
- **25.5 [MAY]** Show where recurring characters/locations appear across episodes; no large continuity DB required.
- **25.6 Season Board:** episode cards reorderable via drag/drop.
- **25.7 Duplicate episode:** optional copy of story/screenplay/production content; new episode identity created [REQ].
- Story-day default scope: episode (§14.5).

## §26 Breakdown Workspace (L998–1034)

ID refs: **FSD-BRK-001 P0** (breakdown from selected screenplay version), **FSD-BRK-002 P0** (manual tagging → scene-specific associations; tag prop/location/cast).

- **26.1 Entry:** select a screenplay source → "Breakdown". Source version displayed at top of Breakdown (§53.1).
- **26.2 Layout:** left = screenplay scene list; selecting a scene shows script content + breakdown elements.
- **26.3 Default categories — exactly, in order:** 1 "Cast", 2 "Extras / Background", 3 "Location / Set", 4 "Props", 5 "Wardrobe", 6 "Vehicles", 7 "Hair / Makeup", 8 "Special Effects", 9 "VFX", 10 "Sound", 11 "Animals".
- **Entity: BreakdownElement** (scene association) — scene id, category, catalog item id, matched/selected text span (optional), source version.
- **26.4 Manual tagging:** highlight/select text OR create manually → choose category → choose existing catalog item or new item.
- **26.5** Existing catalog item selection reuses same production identity; new → added to production catalog.
- **26.6** Scene view shows only relevant categories; empty categories collapsed/hidden.
- **26.7 Remove element [REQ]:** removes only scene association; catalog item NOT deleted.
- **26.8 Catalog deletion:** must handle all scene associations; default = archive, not destructive removal.

## §27 Automatic Breakdown Suggestions (L1037–1069)

ID ref: **FSD-BRK-003 P0** (suggestions require confirmation — accept/reject/edit).

- **27.1 Trigger:** in Breakdown with scene open → "Suggest Elements". [MAY] also offered after import or when a scene is newly selected.
- **27.2 Output:** grouped by category with matched text. Format example: **Prop — pistol** — matched phrase: "carrying a pistol."
- **Entity: BreakdownSuggestion** — category, name, matched phrase, scene, source revision, state {pending, accepted, rejected/dismissed}, optional plain-language confidence.
- **27.3 Actions per suggestion:** "Accept", "Reject", "Edit".
- **27.4 Accept:** creates or links production element; if matching catalog item exists, offer it as first choice.
- **27.5 Reject:** removes from current scene; may remain dismissed for that scene/source revision (i.e., not re-suggested for same scene+revision).
- **27.6 Edit:** change category, name, or association before accepting.
- **27.7 [MAY]** plain-language confidence indicator; never raw model scores.
- **27.8 [REQ]** suggestions never become confirmed items without user action. (AI suggestions enter same flow — §42.9.)

## §28 Production Catalog (L1072–1121)

ID ref: **FSD-CAT-001 P0** (confirmed elements reusable as catalog items — same prop across two scenes).

- **28.1** Reusable production identities, from breakdown or created manually.
- **28.2 Catalog categories (at minimum):** "Cast", "Locations", "Props", "Wardrobe", "Vehicles", "Makeup/Hair", "Special Effects", "VFX", "Sound", "Animals", "Extras". (Names differ from §26.3 — see ambiguities.)
- **Entity: CatalogItem** — required: Name, Category, Status; optional: Description, Image, Notes, Contact/reference; archived flag.
- **28.4 Usage view:** every item shows "Used in Scenes"; clicking a scene opens its breakdown.
- **28.5 Duplicate handling [SHOULD]:** on new breakdown item, search likely existing matches by name/category before creating; ambiguous matches require user confirmation [REQ].
- **28.6 Status values:** "Required", "Searching", "Shortlisted", "Confirmed", "Not Required". UI may reduce the set for simple projects.

## §29 Locations (L1124–1155)

ID ref: **FSD-LOC-001 P1** (locations display scene usage).

- **Entity: Location** — Name, Area/address, Contact, Status (listed as base fields); optional: Photos, Access notes, Parking notes, Noise notes, Power notes, Permission notes, Travel notes; scene associations; replacement mapping.
- **29.2** Location page shows scenes requiring the location.
- **29.3 Status flow:** `Idea → Shortlisted → Confirmed → Rejected`.
- **29.4 Replacement:** if Location A rejected, user sets Location B as practical replacement for *selected* scenes → updates production associations only; NOT screenplay text (unless user separately edits explicit location names) [REQ].
- **29.5 Photos:** available offline; available for mood/reference export.

## §30 Cast and Crew (L1157–1190)

ID ref: **FSD-CAST-001 P1** (cast records connect actors to characters and scenes).

- **Entity: CastRecord** — required: Person name, Character association; optional: Contact, Photo, Availability notes, General notes.
- **Entity: CrewRecord** — required: Person name, Role/department; optional: contact, notes.
- **30.3** One character → one primary actor by default; additional performers (double casting, alternates) representable; UI simple.
- **30.4** Availability = notes only; no HR/payroll engine [REQ — excluded].
- **30.5** From Cast → scenes involving the character; from scene → required cast.
- **30.6 Conflict check:** when scheduling, if same cast member required in conflicting places/times per entered data → schedule warning (see §36).

## §31 Moodboards (L1193–1218)

- **Entity: Moodboard** — name; items (image | note | link) with position, size, order, optional short caption; internal/private notes.
- **31.1 Create:** click "+ Moodboard" → enter board name → add images/notes/links.
- **31.2 Suggested names (suggestions only):** "Overall Look", "Cinematography", "Production Design", "Costume", "Lighting", "Character", "Location".
- **31.3 Canvas:** items draggable, resizable, reorderable; freeform but simple.
- **31.4** Each visual item may have short caption.
- **31.5 Export:** clean PDF/image presentation; excludes internal/private notes unless selected.

## §32 Storyboards (L1221–1259)

ID ref: **FSD-STB-001 P1** (panels associable with shots/scenes).

- **32.1** Visual shot-planning tool; not illustration software.
- **32.2 Create:** Production → Storyboards, or directly from a scene/shot list.
- **Entity: StoryboardPanel** — required: visual (image/sketch/placeholder), shot number, short description; optional: framing, camera movement, angle, dialogue/sound note, duration, storyboard note; order; linked shot; linked screenplay scene.
- **32.4 Add visual:** import image; draw/sketch [LATER: "if supported"]; blank placeholder.
- **32.5 Reorder:** drag panels; shot numbers recalculated from order for that scene/storyboard.
- **32.6** Storyboard may associate to a screenplay scene; must NOT require the screenplay to contain shot directions [REQ].
- **32.7 Export:** clean storyboard sheet/PDF with optional shot notes.

## §33 Shot Lists (L1262–1297)

ID ref: **FSD-SHOT-001 P0** (shots ordered by drag/drop and tied to scenes).

- **33.1** Practical camera-planning doc built scene by scene.
- **33.2 Create:** select scene → "Add Shot".
- **Entity: Shot** — required: short description; computed: shot number (from order); optional: shot size/framing, movement, angle, lens, camera notes, characters, storyboard image, sound note, reference image; scene identity link; linked storyboard panel.
- **33.3** Drag reorder updates order and displayed identifiers.
- **33.4** Attach existing storyboard panel OR create new panel from a shot.
- **33.5** Reference image = visual aid, not an asset-management relationship.
- **33.6 Export:** by scene, group of scenes, or whole project.

## §34 Script → Shot/Storyboard Workflow (L1300–1324)

- Paths (none requires the other module) [REQ]:
  - A: Screenplay scene → Shot List → Storyboard.
  - B: Screenplay scene → Storyboard → Shot List.
  - C: Screenplay scene → Shot List only.
- **34.1 Quick actions on a screenplay scene in Production:** "Breakdown", "Shot List", "Storyboard", "Moodboard reference".
- **34.2 [REQ]** Scene heading and scene identity come from screenplay source when a production object is created from it (no re-entry).
- **34.3** Shots/storyboards link to scene identity, NOT raw line positions. If scene content changes → flag scene for user review; never delete visual planning [REQ].

## §35 Stripboard / Shooting Schedule (L1327–1370)

ID refs: **FSD-SCH-001 P0** (scenes enter Unscheduled on schedule creation), **FSD-SCH-002 P0** (scenes dragged into days).

- **35.2 Initial state [REQ]:** schedule created from a screenplay source → ALL scenes enter **"Unscheduled"**.
- **Entity: Strip (scheduled scene)** displays: generated screenplay scene number; INT/EXT; location; day/night; page count; synopsis/description; optional cast indicators; important breakdown indicators.
- **Entity: ShootingDay** — date, day number, optional notes, ordered strips, day-break markers, entered day duration (implied by §36.4), call sheet status.
- **35.4 Create day:** date; day number; optional notes.
- **35.5 Drag Unscheduled → day:** schedules it; day recalculates scene count, estimated time, cast requirements, location summary.
- **35.6** Reorder within day = production order for that day.
- **35.7 Move between days:** reassigns scene; any existing call sheet generated from the affected day(s) marked as needing update [REQ].
- **35.8** Scene may remain unscheduled indefinitely; filter "only unscheduled".
- **35.9 Day break markers:** meal break; travel; company move; custom note.
- **35.10 [REQ]** No forced optimizer; assistance advisory; human arrangement authoritative.

## §36 Scheduling Assistance and Conflicts (L1373–1397)

ID ref: **FSD-SCH-003 P1** (basic conflicts displayed without forced correction).

- **36.1 [MAY]** Suggest groupings by shared location, cast, or entered shooting duration.
- **36.2 Example:** Scenes 12, 18, 21 share a confirmed location → suggest grouping.
- **36.3 [REQ]** Accepting a suggestion moves scenes only after explicit confirmation.
- **36.4 Warning types:**
  - same actor in different locations on same day;
  - required location scheduled in two incompatible places on same day;
  - scene assigned to a day missing required location information;
  - call sheet generated from a day that has changed since last export;
  - estimated time exceeds entered day duration.
- **36.5** Warnings non-blocking by default; blocking only if user explicitly enables strict validation (project setting).
- **36.6 [REQ]** Every warning has a "Keep Anyway" path.

## §37 Schedule Calendar and Daily Plan (L1400–1422)

- **37.1 Calendar:** shows shooting dates and off days.
- **37.2** Shooting date editable from day details; date belongs to ShootingDay record.
- **37.3 Daily summary:** scenes; locations; cast; estimated duration; notes; call sheet status.
- **37.4 Duplicate day:** creates a similar planning template; scenes retain identity (not duplicated).
- **37.5 Reschedule:** changing date updates the day and marks its call sheet as needing refresh if already exported/prepared.

## §38 Call Sheets (L1425–1463)

ID refs: **FSD-CALL-001 P0** (generated from shooting day data), **FSD-CALL-002 P0** (call sheet edits do not rewrite schedule).

- **38.2 Create:** from schedule, select Shoot Day → "Create Call Sheet".
- **38.3 Default sections and fields:**
  - Production: title, date, shooting day number, crew call.
  - Cast: actor, character, call time.
  - Scenes: scene number, heading, short description.
  - Location: name, address.
  - Practical: parking, meeting point, travel notes, meal/break, emergency contact, production notes.
  - Optional: weather, reference images, attachments, special notes.
- **38.4** Optional sections hidden until added; user can add extra fields.
- **38.5 [REQ]** Scene, cast, location pre-populated from shooting day + project data; no re-entry.
- **38.6 Editable document-specific fields:** call time, notes, meeting point, attachments, others.
- **38.7 No reverse sync [REQ]:** editing call sheet does NOT alter schedule. If schedule changes after creation → call sheet shows **"Schedule changed — Refresh"**; never silently rewritten.
- **Call sheet states (derived):** Draft/editable → needs-refresh ("Schedule changed — Refresh") → "Final/Issued"; replaceable by new revision after issue.
- **38.8 "Final/Issued":** clear document state; does not prevent later replacement with a new revision.
- **38.9 Export:** PDF required [REQ]; snapshot for ordinary channels.

## §39 Lightweight Budget Snapshot (L1466–1500)

- **39.2 Categories (simple):** "Cast", "Crew", "Locations", "Equipment", "Art/Props", "Travel/Transport", "Food", "Post/Other", "Contingency".
- **Entity: BudgetLineItem** — category, description, amount.
- **39.3** Each category holds a small list of line items (description + amount).
- **39.4 Summary:** planned total; optional contingency; current entered total.
- **39.5 Excluded [REQ — must NOT build]:** payroll calculations, tax, union rules, accounting ledgers, purchase orders, invoices, cost reports, enterprise cost codes.
- **39.6 [REQ]** Advisory only. Screenplay change may prompt budget review; must NOT change any monetary value without user confirmation.

## §40 Project Files (L1503–1525)

- **40.1** Project-level file cabinet for docs not represented elsewhere.
- **40.2 Operations:** "Add file"; "Rename display name"; "Move to folder"; "Open externally"; "Reveal in file manager"; "Export/copy"; "Delete to recoverable state".
- **Entity: ProjectFile** — display name, folder, storage mode (stored-in-project | external link/reference), external path, deleted state.
- **40.3** No duplicate media: image already attached to a Moodboard must not produce an unnecessary second user-visible copy in Files.
- **40.4 [REQ]** User can distinguish physically stored vs external link/reference (where supported).
- **40.5** Externally referenced file changes → refresh/relink warning when relevant module opened.

## §41 Global Search (L1527–1562)

- **41.1 Scope:** current project by default; optional toggle to include Global Idea Vault.
- **41.2 Searchable:** Ideas; Acts; Sequences; Beats; Scene Cards; Screenplay text; Characters; Locations; Catalog items; Cast/Crew; Shots; Storyboards; Call sheets; Project files; Comments.
- **41.3 Result format:** type + context, e.g.:
  - `red motorcycle — PROP — Production`
  - `red motorcycle — Scene 25 — Screenplay`
  - `red motorcycle — Idea Vault — Visual Reference`
- **41.4** Click result → open owning page, focus matching item when possible.
- **41.5** Plain text matching sufficient; no semantic search required.
- Private notes visible only to owner (§46.7) → must not leak through search (implied by visibility rule).

## §42 AI Assistant (L1565–1785)

ID refs: **FSD-AI-001 P1** (AI optional; core works with AI disabled), **FSD-AI-002 P1** (AI modifications require confirmation). Full AI detail lives in the AI Specification.

- **42.1** Optional system-wide NL assistant + command interface over existing app. NOT a second database; NOT autonomous. Model interprets, requests app operations, explains. Canonical data, deterministic calculations, validation, permissions, mutations = application responsibilities.
- **42.2** Core functions work without AI [REQ]. Local model may work offline; external providers may need network.
- **42.3 Knowledge:** Product knowledge (terminology, workflows, source-of-truth rules, permissions, versioning, import/export, offline, commands) + Project knowledge (user-accessible current project content + explicitly selected Global Vault content). Prefer authoritative app rule/value over model memory.
- **42.4 Scope selector values:** "Current selection", "Current scene", "Current screenplay", "Specific draft", "Story Board", "Selected Idea Vault items", "Production", "Specific shooting day", "Call Sheet", "Whole project", explicitly requested supported combinations. Resolved scope visible when it materially affects result [REQ].
- **42.5 Read-only Q&A:** allowed without change. Examples: "Summarize this scene."; "Which scenes contain the police station?"; "What changed between these drafts?"; "What locations are used in Act 2?"; "How many characters are in Draft 8?"; "How many scenes are unscheduled?"; "How many locations are used in the current screenplay?". Exact counts/stats/structured facts MUST come from deterministic app queries; model must not guess [REQ].
- **42.6 Navigation/read-only commands:** open workspace; open scene; open draft; show search results; apply supported filters; locate object. Must route through same capabilities as normal UI.
- **42.7 [REQ]** Any create/modify proposal → proposed **Change Set** → mutation preview flow. Imperative phrasing never skips confirmation.
- **42.8** AI may propose Scene Cards, Beats, synopsis, story structures; non-authoritative until accepted; Story Board changes never silently rewrite screenplay.
- **42.9** AI breakdown suggestions stay distinct from confirmed elements; same acceptance flow as §27.
- **42.10 Schedule:** may explain conflicts, find groupings, prepare proposed changes. Must NOT silently: reorder scenes; move scenes between days; change dates; remove scenes; alter schedule assumptions.
- **42.11 Call sheet:** may prepare draft from existing Shooting Day and explain differences vs schedule. Creating/updating the project Call Sheet = mutation → confirmation.
- **42.12 Structured rename/replace** must distinguish: canonical object names; structured references; screenplay Character elements; production references; Story Board references; tasks/notes (only when explicitly selected); arbitrary dialogue/action text. Arbitrary screenplay text NOT replaced by default [REQ].
- **42.13 Batch preview must show:** total affected objects; affected modules; exclusions; locked/unauthorized targets; conflicts; exact operation categories.
- **42.14** AI inherits user's effective permissions; must NOT bypass: Viewer/Commenter/Editor/Owner restrictions; private-note visibility; locked screenplay behavior; archived/deleted lifecycle rules; approval gates; collaboration conflicts.
- **42.15 Change Set base-version:** retains base version/state; if project changed before apply → revalidate, may need new preview/conflict review; never blindly apply stale proposal [REQ].
- **42.16 Tool boundary:** no unrestricted storage write access. Tools validate, in order: 1 target; 2 scope; 3 parameters; 4 user permissions; 5 object/state restrictions; 6 confirmation state → then normal app mutation.
- **42.17 AI result states (minimum):** "Informational", "Pending Approval", "Accepted", "Rejected", "Applied", "Stale", "Conflict", "Failed". (Transitions not specified; implied: Pending Approval → Accepted/Rejected; Accepted → Applied/Failed; any pending → Stale/Conflict on base change.)
- **42.18** Conversation history retained per settings. AI responses never silently become Project Notes, Story Board, or screenplay content. Accepted AI mutations → normal history/activity records, undoable where supported.
- **42.19 External disclosure [REQ]:** before transmission to external provider, UI clearly identifies external processing and project context to be sent. Local AI must not be presented as external/cloud.
- **42.20 Failure safety [REQ]:** on model/tool/provider failure: project unchanged; no partial mutation claimed complete; clear failure/retry path.
- **42.21 Cross-module traversals:** character → scenes → locations; scene → breakdown → catalog; scene → shots → storyboard; scene → shooting day → call sheet; draft → changed scenes → affected production planning. Preserve source-of-truth distinctions in answers.

## §43 Offline and Local-First (L1787–1822)

ID refs: **FSD-OFF-001 P0** (core workflows with network disconnected), **FSD-OFF-002 P0** (local save = source of truth; edit/save/reopen offline).

- **43.1 Offline-capable [REQ]:** create/open projects; edit Vault; edit Story Board; write scripts; create drafts; review/comment locally; create breakdowns; manage catalog; shot lists/storyboards; schedule; call sheets; export supported docs.
- **43.2 Potentially online-only:** external/cloud AI; software update checks; optional external URL metadata retrieval. LAN collaboration discovery does NOT require internet. Configured local AI can stay available offline.
- **43.3 [REQ]** Never block opening offline with a "cannot load until synchronized" message.
- **43.4 [REQ]** Save status refers to local persistence, not cloud.
- **43.5** Projects may live on writable local/external drives; if volume becomes unavailable → clear warning; must NOT silently create a separate inconsistent copy.

## §44 Saving, Autosave, Recovery, Backup (L1825–1853)

- **44.1 Autosave [SHOULD]:** during normal editing; no need to press Save repeatedly.
- **44.2 Manual "Save":** always available; forces persistence.
- **44.3 Crash recovery:** after abnormal shutdown, reopening offers recovery when unsaved/automatic history exists.
- **44.4 Recovery choices:** "Recover latest autosaved state"; "Open last confirmed saved state"; "Compare/review differences" [LATER: "when possible"].
- **44.5 Backup:** user can create a project backup package any time.
- **44.6 Location:** user-selected, changeable; can target another disk.
- **44.7 Content [REQ]:** everything needed to restore supported internal content. External linked files: copied when user requests portable backup, else clearly reported as external references.
- **44.8 Naming:** project name + date/time + optional user label.

## §45 Project Portability and Packages (L1856–1877)

- **45.1** Export portable **Project Package** (whole project, move to another machine).
- **45.2** Project Package ≠ review Exchange Package (full working transfer/backup vs review snapshot).
- **45.3 Import:** "Open/Import Project" → select package.
- **45.4 Identity collision:** if same-identity project exists, ask: "Open as copy" | "Replace existing after backup" | "Cancel". Default favors "Open as copy" [SHOULD].
- **45.5** Import report lists missing external references; project still openable [REQ].

## §46 Roles and Permissions (L1880–1905)

- **46.1 Roles:** "Owner", "Editor", "Commenter", "Viewer", "Export-only".
- **Owner:** change project settings, permissions, content, collaboration session ownership, deletion/restore operations.
- **Editor:** edit allowed content; cannot remove ownership or perform owner-only destructive project operations.
- **Commenter:** view permitted content; add/resolve comments where enabled; cannot edit core screenplay/story/production content.
- **Viewer:** read-only.
- **Export-only:** export selected content; cannot alter project data.
- **46.7 Private notes [REQ]:** visible only to owner (note author); excluded from exchange packages and exports unless explicitly chosen; inclusion should be disallowed for ordinary external review packages.

## §47 Remote Collaboration via Exchange Packages (L1908–1949)

ID refs: **FSD-COL-001 P0** (export a portable exchange package, inspect in another installation), **FSD-COL-002 P0** (review import never blindly overwrites host; test with stale response package).

- **47.1** Files are the transport; no mandatory cloud.
- **47.2 Package types:** "Story Board package"; "Screenplay review package"; "Breakdown package"; "Shot List package"; "Schedule package"; "Call Sheet review package".
- **47.3 Package content (may contain):** selected page/workspace content; source version identity; context needed to understand it; comments (if selected); attachments (if selected); package metadata.
- **47.4 Export:** "Share/Exchange" → "Export Review Package"; UI explains included content before export [REQ].
- **47.5 Receiver import:** "Import Review/Exchange"; app identifies source project/draft/package type.
- **47.6** Receiver comments/annotates per package capabilities → exports response package.
- **47.7** Owner imports response; comments/annotations attach to matching scene/text/package objects where safe.
- **47.8 [REQ]** Import never silently overwrites host project.
- **47.9 Stale package:** if package source older than host content → show source version and current version; default action = import as separate review record or comments-only.

## §48 Local-Network Real-Time Collaboration (L1952–1986) — optional

ID ref: **FSD-COL-003 P1** (LAN session without OpenFrame cloud; host and join on same LAN).

- **48.2 Host:** "Start Collaboration Session" → app displays session name + join info.
- **48.3 Join:** another OpenFrame desktop joins using displayed local session info.
- **48.4 Shared scope (host chooses):** entire project; Story Board; Screenplay; specific production workspace. Default = current project, permissions from session role selection.
- **48.5** Participants edit shared workspace; changes appear to others while connected.
- **48.6 [MAY]** Presence (who/where) — informational only.
- **48.7 Disconnect:** participant's last acknowledged changes stay in session; unacknowledged edits retained locally where possible and exportable as a recovery package.
- **48.8 End session:** host ends; host project remains local and saved.
- **48.9 [REQ]** Ending a session never publishes/uploads to an OpenFrame server.
- **Session states (derived):** not started → hosting/active → participant connected/disconnected → ended.

## §49 Collaboration Conflict Rules (L1989–2016)

ID ref: **FSD-COL-004 P1** (same-object conflicts surfaced, not silently lost).

- **49.1** Different objects → auto-merged.
- **49.2** Same object, different independent fields (e.g., description vs note) → keep both.
- **49.3** Same screenplay text region, not safely combinable → visible conflict state [REQ].
- **49.4 Conflict UI shows:** version A; version B; current working copy; actions "keep A", "keep B", "manually combine".
- **49.5 [REQ]** No silent last-write-wins.
- **49.6** Offline-then-reconnect: collaborator's later local changes must NOT auto-overwrite host; reconcile via same safe conflict handling as exchange packages.

## §50 Exchange Package Types and Merge Rules (L2019–2051)

| Package | Contains | Import behavior |
|---|---|---|
| Story Board | selected Acts/Sequences/Beats/Scene Cards + exporter-selected comments/attachments | preview additions/changes; preserve host objects by default; allow copy-in as new outline OR apply selected changes where mapping unambiguous |
| Screenplay review | source draft identity, content snapshot, comments/annotations | default: attach review comments to matching draft; do NOT replace current draft |
| Breakdown | selected scene breakdown + catalog references | add/update production associations after preview |
| Shot List | selected shots, scene identity, optional storyboard links | update/add shot planning only after confirmation |
| Schedule | shooting days, scene assignments, day notes | must NOT delete local schedule days without explicit user action |
| Call Sheet review | call sheet snapshot + review comments | adds feedback to call sheet record; does NOT modify main schedule |

## §51 Undo/Redo, Multi-Select, Drag-and-Drop, Keyboard (L2054–2097)

- **51.1 Undo [REQ] across major workspaces:** text edit; card create/delete/move; board reorder; breakdown association; shot reorder; schedule move; note change; form edits.
- **51.2 Scope:** current editing session; can restore recent saved state changes. Major imported package actions = single undoable transaction where practical [SHOULD]. Accepted AI mutations undoable where supported (§42.18).
- **51.3 Multi-select** in visual boards and lists. Use cases: move several scene cards; export selected scenes; apply tag/collection; add selected scenes to a production batch.
- **51.4 Drag:** show insertion target or destination container; no precision needed beyond visual card/strip area.
- **51.5 Shortcuts (must include):** Save; Undo; Redo; Find; New; Duplicate (where applicable); Delete; Navigate previous/next scene; Switch common screenplay elements. [LATER] exact key map follows OS conventions, "can be documented separately".

## §52 Delete, Archive, Restore, Destructive Safety (L2100–2119)

- **52.1 Soft delete [REQ]:** important creative/production objects → recoverable deleted state.
- **52.2 Permanent delete [REQ]:** second deliberate action + clear warning.
- **52.3 Relationship safety [REQ]:** deleting a source does NOT destroy downstream objects unless user explicitly confirms cascade. Example: deleting Scene Card does not delete screenplay scene created from it.
- **52.4** Archive preferred for reusable catalog items, projects, people. Delete reserved for accidental/unwanted records.
- **52.5 Restore:** returns object to previous container/order where possible; if container gone → restored to an **Unassigned** area and user notified [REQ].

### Boundary note — §53 begins at L2122 (inside requested range; rest in Part 2)
- 53.1 Breakdown created from chosen draft/revision; source version displayed at top of Breakdown.
- 53.2 When production planning begins, user can mark a source as the "Production Script Version".
- 53.3 (L2130) Newer approved revision → user can choose "Update Production Source" (details continue past L2130).

---

## Cross-Module Source-of-Truth Summary (consolidated from §§1–52)

| Upstream → Downstream | Mechanism | Must NOT |
|---|---|---|
| Idea Vault → Story | "Move to Story" = copy, optional "Source: Idea Vault" label | live sync; remove original |
| Global Vault → Project Vault | copy (new identity) | edit propagates to global |
| Story Board → Screenplay | explicit "Build Screenplay" (preview, heading check) | silent overwrite on rebuild; card edits rewriting scenes |
| Scene Card delete → Screenplay scene | warning only | cascade delete |
| Character rename → Screenplay text | none (screenplay replace controls separate) | blind text replace |
| Screenplay version → Breakdown | chosen source recorded & displayed | production edits rewriting script |
| Breakdown suggestion → Element | Accept/Edit | auto-confirm |
| Remove scene element → Catalog | association removed | catalog item deleted |
| Location replacement → Scenes | production associations updated | screenplay text changed |
| Script revision → Shots/Storyboards | scene flagged for review | delete visual planning |
| Schedule → Call Sheet | pre-populate; "Schedule changed — Refresh" | silent rewrite of call sheet |
| Call Sheet → Schedule | none | reverse sync |
| Screenplay → Budget | may prompt review | auto-change money |
| AI → any | Change Set + preview + confirm | silent mutation; bypass permissions; stale apply |
| Exchange package → Host | preview, default copy/comments-only | silent overwrite; delete local days |

---

## Contradictions and Ambiguities (with line numbers)

1. **Breakdown vs Catalog category names/order differ** — L1009–1019 ("Extras / Background", "Location / Set", "Hair / Makeup", Cast first, Extras second) vs L1080–1090 ("Extras" last, "Locations", "Makeup/Hair"). Need a single canonical enum + mapping.
2. **Comment permissions use two role vocabularies** — L917 "Writer/Director/Producer/Reviewer" and "Commenter capability" vs L1882–1887 roles Owner/Editor/Commenter/Viewer/Export-only. Unclear whether job titles are roles or labels.
3. **Comment replies** — acceptance matrix FSD-SCRIPT-005 (L2576) tests "add comment, reply, resolve", but §23 (L887–923) never defines replies/threads.
4. **Comment targets omit call sheets** — L893–903 list excludes call sheets, yet Call Sheet review package carries "review comments" attached to the call sheet (L2051). Also excludes characters, catalog items, Vault items, shooting days.
5. **Scene Card "required" description may be blank** — L468: required at creation but "may initially be blank"; unclear if an empty card may be persisted.
6. **"Move to Story" is a copy** — L287–288 label says Move but behavior is copy; target types "Sequence idea", "Character note", "generic Story note" are not defined objects in §2/§7.3.
7. **Beat conversion state** — L456: "retained as a history/reference object or marked converted according to UI choice" vs default "Converted" state; unclear whether this is a user choice or implementer choice.
8. **Build Screenplay "creation mode"** — L721 "according to the chosen creation mode" (planning note vs opening action placeholder); modes and default not defined.
9. **Screenplay vs draft vs "screenplay document" cardinality** — L727–728 "create new screenplay draft" vs "build new screenplay document"; L748–749 "New screenplay" vs "New draft in current screenplay project". Whether a project can hold multiple screenplays (and how "exactly one Current draft", L842, scopes) is unclear.
10. **Deleting the Current draft** — L854: delete is soft; only *permanent* deletion of Current is blocked. Unclear whether soft-deleting the Current draft is allowed and what becomes Current.
11. **Relationship between Current draft, Locked Shooting Draft and Revisions** — L842, L929–946: whether a revision is a named draft (comparable via §22, which only allows "two named drafts", L860), and whether locking/revising changes Current, is not specified. "Approved revision" (L966, L2131) has no defined approval flow.
12. **Project type "Episodic" vs "Series"** — L187 lists both; §25.1 (L972) only defines "Series project → Seasons → Episodes". Distinction undefined.
13. **Initial project status** not specified (L200–213); "Archived" is both a status label (L211) and a separate archive action (L218–222) — unclear if they are the same state.
14. **Location status vs Catalog status** — Location flow Idea→Shortlisted→Confirmed→Rejected (L1148) vs catalog statuses Required/Searching/Shortlisted/Confirmed/Not Required (L1115–1119), while "Locations" is also a catalog category (L1081). Two status sets for the same object type.
15. **Shot number double source** — Storyboard panels have required "shot number" recalculated from storyboard order (L1233, L1253), and Shots compute numbers from shot-list order (L1274, L1288). When a panel is attached to a shot (L1291), which ordering wins is unspecified.
16. **Call sheet "needs refresh" trigger inconsistent** — L1361 (call sheet "already exists"), L1422 ("already exported/prepared"), L1390 ("changed since the last export"), L1457 (after "creating a call sheet"). Exact trigger condition differs.
17. **Online-only list contains an offline item** — L1811 lists "optional local-network collaboration discovery does not require internet" under "Potentially online-only features".
18. **Backup location default** — L1847 "Default backup location is user-selected" — no system default defined for first backup.
19. **LAN shared scope default** — L1966 option "entire project" vs L1971 default "current project"; permissions "inherited from session role selection" but role selection UI undefined.
20. **Private notes** — L1905 "excluded … unless explicitly chosen, which should be disallowed for ordinary external review packages": unclear which package types allow inclusion; "owner" = note author vs project Owner role.
21. **"Unassigned" container** — referenced in L395 (Act delete) and L2119 (restore) and L593 (timeline group) but not defined in the Story Board hierarchy (L356–365).
22. **Story Timeline dependency** — L596 "referenced dependent event" implies event dependencies that no section defines.
23. **Budget totals** — L1492–1494 "planned total" vs "current entered total" — difference/source of "planned" undefined.
24. **Strict validation** — L1394 mentions user-enabled strict validation; no setting location/scope defined.
25. **Episode/Review status values** — Episode "status" (L980) and Review round "status" (L890) have no enumerated values (only "Complete" for review, L920).
26. **Terminology slip** — L573 "Scene Board card" (elsewhere "Story Board").
27. **FSD requirement template not populated** — L41–49 require PRD ID/FSD ID/priority per requirement, but sections 1–52 contain none; IDs only exist in matrices §68–70 (L2559–2610), which cover a subset (e.g., no IDs for Characters, Timeline, Import/Export, Comparison, Episodic, Moodboards, Budget, Files, Search, Backup, Permissions, Undo, Delete).
