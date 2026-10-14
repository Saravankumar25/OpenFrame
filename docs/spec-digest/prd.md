# PRD Digest — OpenFrame Studio (Engineering View)

**Source:** `C:\dev\OpenFrame\OpenFrame_Studio_Mega_PRD_Aligned_Updated.md` (5,736 lines, read in full).
**Citation format:** `§N` = PRD section number; `L1234` = line number in the source file.
**Rule:** Nothing here is invented. Where the PRD is silent, the digest says so. Items marked **[FLAG]** need an explicit decision before implementation. Items marked **[ENG NOTE]** are engineering observations, not PRD content.

---

## 0. How the PRD defines priority

| PRD tier | Definition (L29–45) | Digest mapping |
|---|---|---|
| **P0** | "Product identity / first production-ready release" | = v1 / MVP |
| **P1** | "Important second-level capability… can follow after the core workflow is stable" | = next release after v1 |
| **P2** | "Later expansion… should not block the first serious release" | = later |
| **Deferred** | "Explicitly outside current product scope" | = non-goal |

- The PRD never uses "v1"; it uses "MVP" loosely in three places: §143 L3732 ("That is enough for MVP" for Tasks, which are P1), §185 L4606 ("even if multi-window support is not MVP"), and §199 "MVP Product Test" (L4866).
- The FSD must not redefine priority; every FSD requirement inherits PRD priority; new user-visible capabilities must first be added to the PRD (L47, L4843–4851).
- **Authoritative tier sources, in order:** the Requirement ID Registry (L4781–4841) and the §195 tier table (L4855–4862). Where a feature is described in the body but appears in neither, it is marked **untiered** below.
- The PRD says it does not prescribe languages, databases, APIs, cloud infrastructure, UI frameworks or architecture (L23). The Tauri/React/Rust/SQLite stack is therefore an engineering choice, not a PRD requirement. There is no conflict.

---

## 1. Product scope: feature table

Registry IDs are given where they exist. "Untiered" means the body describes the feature but no registry ID or §195 entry covers it.

### 1.1 Core shell, navigation and project

| Feature | Module | PRD section | Tier | Notes |
|---|---|---|---|---|
| Stable app shell: top bar (project name, search, Help/User), left nav, main workspace, status bar (autosave, project) | Shell | §7.1 L358–383 | P0 (PRD-CORE-001) | Should not open a new window for every small task. |
| Primary nav: 8 items (Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets, Files) | Shell | §7.2 L387–407 | P0 (CORE-001) | Cast, Locations, Props and Wardrobe live inside Production, not in the top-level nav. Conflicts with §222. See §7 C-9. |
| Persistent context: project name, status, current draft, save status, collaboration status | Shell | §7.3 L411–423 | P0 (CORE-001) | |
| Non-gated lifecycle (Idea → … → Shoot) | Project | §8 L427–455 | P0 (CORE-001) | "The lifecycle is a guide, not a prison." There are no formal gates. |
| New Project: title and type are required. Language, genre and creator are optional. | Project | §9.1 L461–480 | P0 (CORE-001) | Types: Feature Film, Short Film, Episodic/Series. |
| After creation, land on Project Home with 4 buttons (Open Idea Vault, Open Story Board, Write Screenplay, Import Screenplay) | Project | §9.2 L484–501 | P0 (CORE-001) | Conflicts with the §109 three-option start. See C-10. |
| Project Home: Continue / Recent / Quick Access / Project Files | Project | §15 L781–826 | P0 (CORE-001) | "Not an analytics dashboard." |
| Project status (Idea, Development, Writing, Rewriting, Locked, Pre-production, Shooting, Completed, Archived) | Project | §121 L3267–3281 | P0 (CORE-001, CORE-003) | The app may suggest a transition; the user decides. |
| Archive a project (stays searchable, can be reopened, is not deleted) | Project | §179 L4477–4487 | P0 (CORE-001) | |
| Home shows recent, pinned and archived projects (cards show title, type, status, last modified) | App Home | §180 L4491–4504 | P0 (CORE-001) | |
| Global project search across 12 object kinds, with results labelled by source | Search | §98 L2758–2787; §169 L4282–4294 | P0 (CORE-002) | Separate from in-script find. The user chooses the scope. |
| Files module ("project file cabinet", not a DMS) | Files | §99 L2791–2808 | P0 (CORE-002) | |
| Undo/redo throughout the creative workspace | Core | §100 L2812–2825 | P0 (CORE-002) | Must cover drag/drop, card delete/move, text edits, schedule order, and breakdown add/remove. |
| Delete moves to Recently Deleted; permanent delete needs a second action | Core | §101 L2829–2839 | P0 (CORE-002) | |
| Multi-select (story cards, scenes, shots, Vault items, strips) with move, duplicate, export, delete, tag | Core | §102 L2843–2863 | P0 (CORE-002) | |
| Keyboard shortcuts (new card, save, undo, redo, search, next/prev scene, create draft, add comment, export) | Core | §103 L2867–2884 | P0 (CORE-002) | Keybindings must be "documented and customizable later". Customization is untiered. |
| Desktop drag-and-drop (files into Vault or moodboards, cards, schedule scenes, shots, images into storyboards) | Core | §104 L2888–2899 | P0 (CORE-002) | Must show the destination while dragging. |
| Production Dashboard (script status, breakdown progress, unresolved locations/cast, schedule progress, next shoot day, latest call sheet, warnings) | Production | §120 L3244–3263 | P0 (CORE-003) | "No giant KPI wall." |
| Useful empty states (copy is specified for Story Board, Screenplay, Breakdown, Schedule) | UX | §126 L3374–3398; §194 L4767–4777 | P0 (CORE-003) | |
| Error messages say what happened, why it matters, and what the user can do | UX | §127 L3402–3418 | P0 (CORE-003) | |
| Document identity block on exports (project title, doc title, date, page numbers, revision) | Export | §187 L4624–4636 | P0 (CORE-003) | The user can customize a small identity block. |
| Beginner/professional paths, progressive disclosure, page UX rules | UX | §109–111 L2981–3050; §188–194 L4640–4777 | P0 (PRD-UX-001) | No setup wizard (§193). |
| Confirmation required for destructive actions (delete project/draft, permanently delete card, replace screenplay on import, overwrite from exchange package) | Core | §128 L3422–3439 | Untiered | Non-destructive actions must **not** ask for confirmation. |
| "Open in…" contextual navigation for every major object | Core | §170 L4298–4315 | Untiered | |
| Scene Hub (screenplay, breakdown, shots, storyboard, schedule day, comments per scene) | Core | §171 L4319–4343 | **P1** (§195 L4860 only; not in registry) | §189 L4696 (UX-001, P0) relies on it. See C-13. |
| Sequence Hub | Story | §172 L4347–4356 | Untiered | |
| `+` Quick Actions menu (10 create/import actions, contextual) | Shell | §173 L4360–4378 | Untiered | AI entry routes to the same actions (L4364). |
| Consistent right-click context menus (scene card, screenplay scene) | UX | §174 L4382–4405 | Untiered | |
| Quick Capture: global action that saves a note to the current project Vault | Idea | §175 L4409–4423 | Untiered | "Can be implemented as a desktop-level command later." |
| Resizable, maximized and fullscreen-writing windows; optional secondary windows | Shell | §184 L4569–4578 | Untiered | Core work must stay possible in one window. |
| Multi-monitor / separate windows | Shell | §185 L4582–4606 | **P1** (§195) | "Not MVP" (L4606). |
| Fullscreen Writing mode | Script | §186 L4610–4620 | Untiered (related Focus Mode is P0 under SCRIPT-001) | |
| Templates (project, call sheet, shot list, breakdown, moodboard, storyboard) | Templates | §122 L3285–3300 | **P1** (PRD-TPL-001) | Users can save layout preferences. |
| Richer project search | Search | ref. §197 (section missing) | **P2** (PRD-CORE-004) | |

### 1.2 Idea Vault

| Feature | Module | PRD section | Tier | Notes |
|---|---|---|---|---|
| Global Idea Vault plus an auto-created Project Idea Vault for each project | Idea | §10 L505–572; §14 L757–777 | P0 (PRD-IDEA-001) | Items are **copied** between Vaults, not live-linked (L570–572). |
| Item types: text, image, URL, PDF, document, audio, voice note, video, folder/collection, sketch image, quote, screenshot, "any ordinary file" | Idea | §10.2 L526–546; §11 | P0 (IDEA-002) | Titles are never required. Files must not be rejected for lack of a category (§11.5). |
| Note creation: cursor goes straight into the body; saving needs no category | Idea | §11.1 L602–616 | P0 | Acceptance: "create a note in one interaction" (L4912). |
| Image card with optional title, caption, tags, collection, source link | Idea | §11.2 L620–638 | P0 | |
| URL item: URL, title "if available", user note, optional screenshot/thumbnail | Idea | §11.3 L642–655 | P0 | Implies an outbound fetch. See §6 A-11. |
| Voice note recording; optional STT that never replaces the audio | Idea | §11.4 L659–671 | P0 (recording). STT is untiered ("if available"). | See §6 A-10. |
| Views: Visual Grid, Card, List, Folder; last view remembered | Idea | §10.4 L576–596 | P0 (IDEA-003) | |
| Organization: folders, collections, tags, favorites/pins, search, recently added, recently modified | Idea | §12 L687–713; §176–177 L4427–4459 | P0 (IDEA-003) | Pinned items appear at the top. The app must not prescribe folder names. |
| Copy Global → Project (the project copy becomes independent); Project → Global | Idea | §178 L4463–4473; §14 L777 | P0 (IDEA-003) | |
| Vault search, preview and external-file awareness | Idea | Registry L4791 (§11, 98, 107) | P0 (IDEA-004) | Related cross-cutting rule: behave stably when external files or drives are unavailable (L188). |
| Send to Story Board (as Beat, Scene Card, Sequence idea, Character note or Story note): creates a copy, the original stays, and a "Used in Story Board" reference appears | Idea→Story | §13 L717–753 | P0 (IDEA-005) | There is no sync back to the Vault. |
| Research collections | Idea | §140 L3660–3677 | Untiered | "Not a formal academic citation manager." |

### 1.3 Story (outline)

| Feature | Module | PRD section | Tier | Notes |
|---|---|---|---|---|
| Board View and Outline View over one shared story order | Story | §17 L840–878 | P0 (PRD-STORY-001) | |
| Acts (title only; optional description; may contain Sequences or Scene Cards directly) | Story | §18 L882–906 | P0 (STORY-001) | Scene Cards directly under an Act are explicitly for short films. |
| Sequences (name only; optional note; no "purpose" field) | Story | §19 L910–932 | P0 (STORY-001) | |
| Beat Cards (text; optional color, note, reference); can be converted to a Scene Card | Story | §20 L936–960 | P0 (STORY-002) | A Beat may never become a scene. |
| Scene Cards: compact; truncated short description; heading only in the detailed view | Story | §21 L964–1009 | P0 (STORY-003) | Heading rules conflict. See C-1. |
| Story Board shows no permanent scene numbers (letters such as Scene A/B/C) | Story | §22 L1013–1035 | P0 | Screenplay numbers come from screenplay order. |
| Scene Card expansion by double-click (heading, description, notes, attachments) | Story | §23 L1039–1063 | P0 (STORY-003) | No production metadata by default. |
| Drag-and-drop: 9 required moves, visible insertion point, no confirmation, undo | Story | §24 L1067–1089 | P0 (STORY-004) | |
| Parking Lot (removes a card from active order without deleting it) | Story | §25 L1093–1111 | P0 (STORY-004) | |
| Duplicate cards; no auto-merge | Story | §26 L1115–1133 | P0 (STORY-004) | |
| Build Screenplay from active Scene Cards | Story→Script | §27–29 L1137–1202 | P0 (STORY-005) | One-way. Reorder warning conflicts with that. See C-2. |
| Characters (name; optional role, description, image, notes, relationship notes; auto-derived "Scenes this character appears in") | Story | §30 L1206–1225 | P0 (STORY-006) | Scene links come from screenplay parsing, not manual entry. |
| Character Relationship View | Story | §31 L1229–1241 | P0 (STORY-006) | Body says "the relationship map is optional" (L1239). See C-14. |
| Story Timeline (story-day grouping) | Story | §32 L1245–1266 | **P1** (STORY-007) | Listed under "Core creative surface" in §229 (L5624). |

### 1.4 Screenplay

| Feature | Module | PRD section | Tier | Notes |
|---|---|---|---|---|
| Professional screenplay editor with elements Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, Shot, and non-printing general text/notes | Script | §33–34 L1270–1291 | P0 (PRD-SCRIPT-001) | "Never manually adjust spacing/alignment." |
| Element navigation (shortcuts, compact selector, context rules such as Heading+Enter → Action and Character+Enter → Dialogue) | Script | §35 L1295–1311 | P0 | Follow industry-standard keys "where practical". |
| Layout modes: Focus, Normal, +outline sidebar, +scene notes panel | Script | §36 L1315–1326 | P0 | |
| Scene navigator panel with click-to-jump | Script | §37 L1330–1351 | P0 | |
| Find/replace (text, next/prev, replace, match case, whole word), current screenplay only | Script | §38 L1355–1369 | P0 | "Should not build an overpowered semantic writing search." |
| Non-printing scene notes with a visibility toggle | Script | §39 L1373–1383 | P0 | Excluded from PDF by default. |
| Side panels (Story Board, Scene Notes, Characters, Comments, Draft info); 1–2 open at a time; layout remembered | Script | §40 L1387–1403 | P0 | |
| The Idea Vault is **not** an embedded writing sidebar | Script | §41 L1407–1421 | P0 (constraint) | |
| Import PDF, FDX, Fountain, TXT, DOCX, pasted text | Script | §42 L1425–1440 | P0 (SCRIPT-005) | |
| Import preview (headings, characters, page count, detected format), then "Import as New Screenplay" or "Add as New Draft" | Script | §43 L1444–1467 | P0 (SCRIPT-005) | |
| Import safety: never auto-replace; create a new draft if a screenplay already exists | Script | §44 L1471–1477 | P0 (SCRIPT-005) | |
| Export PDF, FDX, Fountain, DOCX, preserving headings, names, dialogue, action, page breaks, scene numbers, title page, revision info | Script | §45 L1481–1499 | P0 (SCRIPT-006) | |
| Printing on standard paper sizes | Export | §125 L3358–3370 | P0 (SCRIPT-006) | Paper size is not specified. See C-24. |
| Automatic edit history (restore points) plus named drafts | Script | §46 L1503–1525 | P0 (SCRIPT-002) | No restore-point interval is given. |
| New Draft from the current draft or a previous named draft, with name and optional note; the old draft is unchanged | Script | §47 L1529–1550 | P0 (SCRIPT-002) | |
| Draft comparison summarized per scene ("Scene 17 changed", "12 lines changed in Scene 34"), then exact diff | Script | §48 L1554–1580 | P0 (SCRIPT-002) | Not a raw machine diff. |
| Review Rounds (source draft, reviewers, comments, status, optional deadline, final state; open/resolved counts) | Script | §49 L1584–1605 | P0 (SCRIPT-003) | |
| Comments on 9 target kinds (script text, scene card, beat, sequence, act, storyboard panel, shot, location, breakdown item); Open or Resolved; resolved comments stay in history | Collab | §50 L1609–1632 | P0 (SCRIPT-003) | |
| Private notes (never shared, visually distinct, never exported to review packages or PDF) | Script | §51 L1636–1646 | P0 (SCRIPT-003) | |
| Script Lock ("Locked Shooting Draft", with a pre-lock checklist) | Script | §52 L1650–1668 | **P1** (SCRIPT-004) | Lock can be undone by authorized users. |
| Editing a locked draft prompts "Create a revision?" | Script | §53 L1672–1687 | **P1** (SCRIPT-004) | Locking does not make the draft immutable. |
| Production revision colors and revision pages; history shows name, date, reason, changed scenes | Script | §54 L1691–1710 | **P1** (SCRIPT-004) | The color is metadata and export styling only. |
| Additional file interchange formats | Script | ref. §197–198 (sections missing) | **P2** (SCRIPT-007) | |

### 1.5 Episodic

| Feature | Module | PRD section | Tier | Notes |
|---|---|---|---|---|
| Series → Season → Episode → Story Board → Screenplay; each episode has its own Vault subset, Story Board, drafts, breakdown and production plan; the series has a lightweight shared character/location reference | Episodic | §55 L1714–1740 | P0 (PRD-EP-001) | "Should not build a giant writers-room database." |
| Series-level Story Board (episode list with optional title, one-line summary, status) | Episodic | §56 L1744–1762 | P0 (EP-001) | |
| Deeper episodic continuity | Episodic | ref. §197 | **P2** (EP-002) | |

### 1.6 Production (pre-production)

| Feature | Module | PRD section | Tier | Notes |
|---|---|---|---|---|
| Breakdown: scene script shown beside a compact breakdown tool; manual tagging | Breakdown | §57–58 L1766–1798 | P0 (PRD-BRK-001) | |
| Automatic breakdown suggestions ("Suggested elements — N"; Accept, Reject or Edit) | Breakdown | §59 L1802–1830 | P0 (BRK-001) | "No suggestion should silently become production data." Must work without AI. See A-15. |
| 11 default categories (Cast, Extras/Background, Location/Set, Props, Wardrobe, Vehicles, Hair/Makeup, SFX, VFX, Sound, Animals); custom categories "only when necessary" | Breakdown | §60 L1834–1850 | P0 (BRK-001) | |
| Production Catalog (reusable entries shared across scenes; fields: name, category, description, image, notes, scenes, status) | Catalog | §61–62 L1854–1891 | P0 (PRD-PROD-001) | Confirmed breakdown elements become catalog entries. |
| Locations (name, address, contact, photos, notes, scenes, status Idea/Shortlisted/Confirmed/Rejected) with practical notes (parking, noise, permission, access, power, toilets, facilities, travel, restrictions) | Locations | §63–64 L1895–1932 | P0 (PRD-LOC-001) | |
| Cast (name, character, contact, photo, notes, availability, scenes); no payroll, union or contracts | Cast | §65 L1936–1954 | P0 (PRD-CAST-001) | |
| Crew directory (name, role, department, contact, notes) | Crew | §66 L1958–1979 | P0 (CAST-001) | "Not HR software." |
| Moodboards (images, colors, notes, text, links; freely draggable) | Visuals | §67 L1983–2005 | P0 (PRD-VIS-001) | |
| Moodboard export (title, board name, images, optional captions; internal notes excluded unless selected) | Visuals | §68 L2009–2020 | P0 (VIS-001) | |
| Basic storyboards (panel with shot number, image or sketch, description; optional framing, movement, dialogue, notes, duration; draw, import or use a placeholder) | Visuals | §69 L2024–2044 | P0 (PRD-STB-001) | "Simpler than specialist illustration software." |
| Script-to-Storyboard: "Create Storyboard" makes empty panels | Visuals | §70 L2048–2058 | P0 (STB-001) | No auto-invented visuals unless AI is asked. See A-13. |
| Better storyboard tools / advanced storyboard editing | Visuals | ref. §196 / §197 | **P1** (STB-002) / **P2** (STB-003) | |
| Shot lists, scene-grouped and drag-reorderable (numbered 24A, 24B, …) | Shots | §71 L2064–2081 | P0 (PRD-SHOT-001) | |
| Shot card: number and description required; 9 optional fields hidden until needed | Shots | §72 L2085–2106 | P0 (SHOT-001) | |
| Optional two-way attachment between shots and storyboard panels; neither requires the other | Shots/Visuals | §73 L2110–2120 | P0 (SHOT-001) | |
| Stripboard: one strip per scene showing number, INT/EXT, location, D/N, page count, synopsis, cast and breakdown indicators | Schedule | §74 L2124–2143 | P0 (PRD-SCHED-001) | "Lighter" than Filmustage. |
| Board view and List view over the same schedule | Schedule | §75 L2147–2161 | P0 | |
| Create Shooting Schedule puts all scenes in Boneyard/Unscheduled; the user drags them into Shoot Days | Schedule | §76 L2165–2183 | P0 | The term "Boneyard" may change. |
| Shooting Day (date, day number, scenes, locations, cast, notes, estimated duration); markers for Meal Break, Travel, Company Move, Custom | Schedule | §77 L2187–2204 | P0 | |
| Basic schedule assistance (e.g., "same location, group them"); suggestions only, never silent rearrangement | Schedule | §78 L2208–2224 | P0 (SCHED-001) | The wording says "assistant". See A-14. |
| Basic conflict detection (e.g., same actor at 2 locations on 1 day); warns only | Schedule | §79 L2228–2252 | P0 (SCHED-001 covers §74–81) | |
| Optional per-scene time estimates rolled up into a day total | Schedule | §80 L2256–2276 | P0 | "No complicated production-time formula." |
| Calendar mapping shoot days to dates, with OFF days | Schedule | §81 L2280–2300 | P0 | The stripboard stays the main tool. |
| Advanced conflict detection | Schedule | ref. §196 | **P1** (SCHED-002) | |
| More sophisticated schedule assistance | Schedule | ref. §197 | **P2** (SCHED-003) | |
| Call sheets generated from a shooting day and pre-populated | Call Sheets | §82 L2304–2312 | P0 (PRD-CALL-001) | |
| Call sheet default contents: production, cast, scenes, location, practical notes; optional weather, attachments, images, notes | Call Sheets | §83 L2316–2358 | P0 | There is no per-department crew call list. See C-23. |
| Call sheets stay editable; edits do not change the schedule unless the user explicitly chooses to | Call Sheets | §84 L2362–2375 | P0 | |
| Call sheet exports as a clean PDF readable on desktop, tablet and phone | Call Sheets | §85 L2379–2389 | P0 | No mobile app. |
| Production handoff: "Start Production Setup" picks a source draft and initializes breakdown scenes, catalog, shot/storyboard containers and scheduling source | Production | §147 L3787–3811 | **Untiered** | Not in the registry. See C-12. |
| Production Source display ("Production Source: Shooting Draft 6") that stays fixed until explicitly updated | Production | §148 L3815–3823 | Untiered | |
| Production Source Update (diff of added, removed and changed scenes and headings; Update / Keep / Review) | Production | §149 L3827–3846 | Untiered | "Critical control point." |
| Breakdown, shot list, storyboard and schedule reconciliation (flag and suggest; never auto-delete or reschedule) | Production | §150–153 L3850–3904 | Untiered (AC §203.8–9 and §207.6 are P0) | |
| Scene identity: persistent ID; number derived from order; production links survive renumbering | Core data | §115 L3137–3151 | Untiered (implied by P0 ACs) | |
| Scene reorder safety warning ("will not automatically reorder the shooting schedule") | Script/Schedule | §116 L3155–3168 | Untiered | |
| Breakdown after script change ("Scene 24 changed. Review breakdown differences"; Apply / Keep / Review) | Breakdown | §117 L3172–3208 | Untiered (AC §203.8–9 P0) | |
| Schedule after script change ("Script changed in 3 scheduled scenes"; Review Changes panel) | Schedule | §118 L3212–3222 | Untiered | |
| Call sheet marked stale ("Based on an older schedule version"); Update Call Sheet with diff | Call Sheets | §119 L3226–3240 | Untiered (AC §207.6 P0) | |
| Basic production reports (scene breakdown, cast/location/props by scene, schedule, call sheet) | Reports | §154 L3908–3921 | **P1** (PRD-PROD-003) | "Do not build dozens of studio reports." |
| Sides per shoot day (all cast, selected cast or selected scenes) as PDF | Reports | §155 L3925–3943 | **P1** (PROD-004) | Listed as "Core pre-production surface" in §229 L5644. |
| Daily Production View ("Today: Shoot Day 4" with scenes, location, call sheet, shot list) | Production | §156 L3947–3973 | **P1** (PROD-002) | "Not a full on-set operations platform." |
| Lightweight Budget Snapshot | Production | **No body section** (registry L4834 cites "140, 154 and budgeting references") | **P1** (PROD-005) | Defined only by the registry. See C-5. |
| Project Notes (freeform), Lightweight Tasks (title, optional assignee, Open/Done, optional due date), Activity History | Project | §141–144 L3681–3752 | **P1** (PROD-006) | §143 says "enough for MVP". See C-15. |
| More production reports | Reports | ref. §197 | **P2** (PROD-007) | |
| "Shooting Complete" marker; post-production files stored as project files | Project | §157 L3977–3985 | Untiered | |

### 1.7 Collaboration, offline and data ownership

| Feature | Module | PRD section | Tier | Notes |
|---|---|---|---|---|
| Mode 1, Solo Local (default, no internet) | Collab | §87 L2405–2411 | P0 (PRD-OFF-001) | |
| Mode 2, Exchange Packages (file-based; no OpenFrame account) | Collab | §87 L2415–2441; §88 L2461–2476 | P0 for review packages (COL-002); P1 for other page types (COL-003) | Package contents: selected content, required context, optional comments and attachments, source version ID, export timestamp. |
| Review Package round trip (Script only, Script + comments, or specific scenes → reviewer annotates → Review Response Package → "14 new comments received" → Import Review) | Collab | §89 L2480–2523; §134 L3561–3576; §133 L3531–3557 | P0 (PRD-COL-002) | Package metadata: project, target, draft, "Exported by", date, contents. Banner: "comments are separate from the original project". |
| Merge safety (older-version packages: view only, import comments only, separate review record, or compare) | Collab | §90 L2527–2542 | Untiered (AC §208.5–6 P0) | Never blindly overwrite. |
| Comment import mapping to draft, scene and text; otherwise "Unmatched Review Notes"; nothing silently discarded | Collab | §135 L3580–3594 | Untiered | |
| Page-specific packages: `.ofstory`, `.ofscriptreview`, `.ofbreakdown`, `.ofshots`, `.ofschedule`, `.ofcallreview` (extensions may change) | Collab | §91 L2546–2578; §136–138 L3598–3646 | **P1** (PRD-COL-003) | Only Story, Shot and Breakdown exchange behaviors are described. |
| Better / richer exchange package controls | Collab | ref. §196 | **P1** (COL-004) | |
| Mode 3, Local Network Collaboration (host machine is the session authority; Start / Join Local Session; host picks project and allowed pages; "Working in local shared session") | Collab | §87 L2445–2457; §131 L3490–3509 | **P0** (PRD-COL-005) | Engineering Index defers shipping. See C-18. |
| Soft locks: show who is editing script text, a scene or a catalog item; "Two people must not unknowingly overwrite each other" | Collab | §132 L3513–3525 | P0 (COL-005) / P1 (COL-006) overlap | See C-17. |
| Advanced collaboration conflict handling | Collab | §131–132 | **P1** (COL-006) | |
| Permissions: Owner, Editor, Commenter, Viewer, Export-only | Collab | §129 L3443–3467 | **P1** (PRD-COL-001) | Does not match the 8 roles in §4. See C-7 and C-16. |
| Offline collaboration state (solo, packages and local files always work; host keeps the latest copy on disconnect; clients informed; no data loss) | Collab | §130 L3471–3486 | P0 (OFF-001) | |
| Autosave with Saved / Saving… / Unsaved changes status; explicit save or export of a project package | Data | §105 L2903–2917 | P0 (OFF-001) | |
| Export Project Archive (structure, Story Board, drafts, breakdown, catalog, production planning, comments, attached files where included) | Data | §106 L2921–2946 | P0 (OFF-001; "project archive" in §195 P0) | Format is an engineering choice. |
| User-chosen storage (local drive, external SSD, project folder, network drive "where supported by the OS"); no OpenFrame cloud folder | Data | §107 L2950–2961 | P0 (OFF-001) | |
| Backups: manual archive, automatic local backup versions, backup location selection; "Project backups are separate from your active project" | Data | §108 L2965–2977 | P0 (OFF-001) | No count or frequency is given. |
| Media handled as references or attachments only; not a media editor or DAM | Data | §182 L4526–4532 | Untiered (constraint) | |

### 1.8 Exports (consolidated from §123–124)

| Group | Formats (§123 L3304–3336) | Tier |
|---|---|---|
| Writing | PDF, FDX, Fountain, DOCX | P0 (SCRIPT-006) |
| Story | Story Board PDF, Outline PDF | P0 (the registry maps §123–125 into SCRIPT-006) |
| Production | Breakdown PDF, Catalog CSV/XLSX-style, Shot list PDF, Storyboard PDF, Schedule PDF, schedule spreadsheet "where practical", Call sheet PDF | P0 (see the per-module acceptance criteria §204.6, §205.6, §206.10, §207.5) |
| Project | Project Archive | P0 |
| Scope selector | Export Current Page / Selected Items / Entire Project (§124 L3340–3354) | P0 |
| Private notes | Never exported by default (L3354; L1646) | P0 |
| More export layouts | — | **P1** (§195 L4860 only) |

### 1.9 AI

| Feature | Module | PRD section | Tier | Notes |
|---|---|---|---|---|
| Optional system-wide natural-language assistant and command interface ("Amazon Q-style"), grounded in OpenFrame knowledge and project knowledge; never authoritative | AI | §93 L2604–2613 | **P1** (PRD-AI-001) | Not a second database. |
| Allowed on explicit request: Q&A, exact counts, draft comparison, locating objects, relationship traversal, navigation, summaries, continuity concerns, Scene Card suggestions, breakdown suggestions, synopsis, schedule-grouping advice, call-sheet drafts, tasks/notes after confirmation, structured rename/replace, batch ops, stale-dependency explanation, product help | AI | §94 L2617–2652 | P1 | Routes to the same actions as the UI; no parallel command system (L2652, L4364). |
| Not allowed silently: rewrite script, change canon, modify cards, rename, replace text, add breakdown items, move schedule, edit call sheets, delete/archive, lock/approve, change permissions, any mutation | AI | §95 L2656–2675 | P1 | "A natural-language imperative is not blanket permission" (L2673). |
| Scope selector (selection, scene, screenplay, draft, Story Board, Vault items, production, shoot day, call sheet, whole project, combinations); scope shown when it matters; ask instead of guessing on ambiguity | AI | §96 L2679–2703 | P1 | Counts must come from deterministic calculation (L2703). |
| Change Set approval flow (resolve → permission check → Change Set → preview → accept → revalidate → apply via normal action → undo/activity/recovery) | AI | §97 L2707–2754 | P1 | Rename preview must separate structured references from arbitrary dialogue/action text, which is not changed by default (L2731–2740). |
| Model target: "small local model, including a Qwen2.5-0.5B-class model"; the model does not do exact calculation, permission decisions, validation or direct storage writes | AI | §97 L2746 | P1 | Consistent with the local-only decision. |
| More advanced AI assistance | AI | ref. §197 | **P2** (PRD-AI-002) | |

### 1.10 Deferred / non-goals (tier table)

§195 L4862 Deferred: accounting, payroll, full budgeting, VFX management, post-production tracking, distribution, festival management, talent marketplace, cloud media hosting, **mobile app**, enterprise administration. See §2 for the full boundary list.

---

## 2. Explicit non-goals and product boundaries

### 2.1 Stated exclusion lists
- **Header scope boundary** (L13): not a studio ERP, accounting platform, VFX tracker, post-production asset manager, distribution CRM, payroll system, or enterprise production-management suite.
- **§158 No post-production in core** (L3989–4004): no editing timeline, media asset review, VFX tracking, color grading, sound mixing, dailies platform, delivery/QC, or distribution CRM. The product ends at "Prepared to shoot / call-sheet-ready / small production execution."
- **§159 What we are not building** (L4008–4032), 20 items: studio accounting; payroll; union management; tax; ERP; full legal contract platform; casting marketplace; equipment marketplace; VFX tracker; post-production asset manager; media server; dailies platform; color pipeline; distribution sales platform; festival CRM; massive rights management; **enterprise SSO/identity suite**; complex departmental permission matrices; full project-management suite; **mandatory AI writing coach**; **automated screenplay scoring**.
- **§229 Explicitly outside the core** (L5658–5666): post-production, VFX management, accounting, payroll, distribution, festival management, enterprise studio management.
- **§231 Mantra** (L5704–5710): "Not an ERP. Not a cloud collaboration company. Not a post-production suite. Not an AI screenplay generator."

### 2.2 Platform boundaries
- Desktop only, on Windows and macOS (L8, §5.1 L254–265). "A mobile application is not part of the initial product." "A browser-only replacement is not the product goal."
- Mobile access is provided **only through exported PDFs** (§85 L2389).
- No OpenFrame-hosted cloud folder (L2961). No OpenFrame cloud needed for exchange or LAN (L2441, L2451). OpenFrame does not own the transport channel (L2600, L3555).

### 2.3 Module-level "do not build" constraints
| Constraint | Location |
|---|---|
| No semantic/"overpowered" in-script search | §38 L1357 |
| No "linked thinking layer" between Vault notes and script paragraphs | §41 L1413 |
| No giant writers-room database for episodic work | §55 L1740 |
| No formal "sequence purpose" field | §19 L928 |
| Character relationships are a visual helper, not a character-analysis system | §31 L1241 |
| Cast: no payroll, union accounting or talent contracts | §65 L1950–1954 |
| Crew is a directory, not HR software | §66 L1979 |
| Storyboards are simpler than illustration software | §69 L2026 |
| No complicated production-time formula | §80 L2276 |
| Schedule is suggest-only, never a forced optimizer | §78, §213 L5212 |
| No dozens of studio reports | §154 L3921 |
| Not a full on-set operations platform | §156 L3973 |
| Not a Jira/Asana competitor (tasks are Open/Done only) | §142 L3714, §143 |
| Activity history is for orientation, not enterprise auditing | §144 L3752 |
| No academic citation manager | §140 L3677 |
| Files module is not a DMS | §99 L2806 |
| Not a media editing or asset-management platform | §182 L4530–4532 |
| No dozens of enterprise roles | §129 L3467 |
| No large setup wizard | §193 L4759 |
| Dashboards are not KPI walls or analytics dashboards | §15 L783, §120 L3263 |

### 2.4 Feature bloat guardrail (§160 L4036–4045)
A feature is rejected or postponed if it: (1) is rarely used by indie filmmakers; (2) requires large metadata entry; (3) mainly serves studio-scale edge cases; (4) duplicates an outside tool; (5) doesn't connect to story or practical pre-production; (6) adds significant UI complexity without comparable value. Also: "If a feature cannot be placed naturally, it may not belong in the core product" (§226 L5578). Rule 15: "No enterprise feature may be added merely because a large studio might someday want it" (L5451).

### 2.5 Behavioral boundaries (source-of-truth and no-sync rules)
- **Connections that must NOT exist** (§113 L3102–3116): Vault↛screenplay auto-sync; Story Board↛auto-rewrite of the screenplay; screenplay dialogue edits↛rewrite of Story Board descriptions; moodboards↛auto-change of production design data; call sheet edits↛auto-change of schedule.
- **Source of truth** (§114 L3120–3131):

| Information | Source of truth |
|---|---|
| Raw ideas | Idea Vault |
| Story order | Active Story Board, **or the screenplay after it is written** |
| Written dialogue/action | Screenplay draft |
| Production elements | Breakdown / Catalog |
| Shooting order | Stripboard |
| Call-sheet presentation | Call Sheet |
| Shot plan | Shot List |
| Visual shot reference | Storyboard |

- **Required connections** (§112 L3054–3098): Vault→Story (optional copy); Story→Screenplay (optional generation); Screenplay→Breakdown (direct extraction); Screenplay→Shot List (scenes become containers); Screenplay→Storyboard (scenes become containers); Breakdown→Catalog (confirmed elements); Catalog→Schedule (cast/location requirements inform planning); Schedule→Call Sheet (day populates the sheet); Storyboard→Shot List (optional reference); Shot List→Call Sheet (optional daily shots).
- **Automation boundary** (§2.4 L153–172). Good: suggest breakdown elements, carry headings into production, generate scene numbers, populate call sheets, create drafts from drafts, generate a synopsis on request. Bad: auto-change story, auto-move schedule scenes, auto-change the screenplay, auto-categorize ideas, auto-delete "unused" material.
- **Engineering handoff priority order** (§230 L5674–5686): (1) simple workflow, (2) ownership/offline, (3) source-of-truth, (4) no duplicate entry, (5) no silent destructive sync, (6) explicit over hidden automation, (7) fewer controls with defaults, (8) advanced behavior behind options, (9) no enterprise complexity without demonstrated need. "If … technically elegant but makes the filmmaker's workflow harder, the workflow wins."
- **Non-negotiable rules** (§221 L5437–5451): 15 rules. The engineering-relevant ones are #4 (no manual scene numbers during outlining), #6 (Story Board is not a synced duplicate of the screenplay), #7 (breakdown without re-entry), #8 (suggestions require confirmation), #9 (drag-and-drop-first scheduling), #10 (call sheets derive from shooting days), #11 (offline core), #12 (ownership/export), #13 (no OpenFrame cloud for remote collaboration), #14 (AI optional and user-controlled).

---

## 3. Project types and per-type availability

**Types** (§9.1 L468–472): **Feature Film**, **Short Film**, **Episodic / Series**. No other types exist (no commercial, music video or documentary). Optional project fields: Language, Genre, Creator.

The PRD does **not** gate features by project type. All modules are available to all types. Per-type behavior is expressed as allowances and "must not force" rules:

| Aspect | Feature Film | Short Film | Episodic / Series |
|---|---|---|---|
| Structure | Acts → Sequences → Beats/Scene Cards | May place Scene Cards directly under an Act with no Sequences (§18 L902–906). Must not be forced into complex acts, detailed breakdown categories or elaborate schedule optimization (§164 L4201–4207). | Series → Season → Episode → Story Board → Screenplay (§55 L1718–1728) |
| Minimum valid project | — | Idea → 5 Scene Cards → Screenplay → Breakdown → Simple Schedule → Call Sheet "is a complete valid OpenFrame project" (§192 L4737–4753). Must not require completing "50 fields" (L4735). | — |
| Reference size | §181 targets (120 pages, 150–250 cards; see §5). "Should not require splitting a normal feature film into multiple projects" (L4522). | Example: 5 scenes, 1 location, 3 actors (§164 L4197–4199) | Not specified |
| Idea Vault | Project Vault | Project Vault | Per-episode "Idea Vault subset" (L1732) |
| Story Board | One per project | One per project (acts optional in practice) | One per episode, plus a **Series-level Story Board** of episodes with optional title, one-line summary and status (§56) |
| Screenplay drafts | Per project | Per project | Per episode (L1734) |
| Breakdown / production plan | Per project | Per project | **Per episode** (L1735–1736). No cross-episode schedule is described. See C-22. |
| Shared references | — | — | Lightweight series-level shared character/location reference (L1738) |
| Continuity | — | — | Deeper episodic continuity is **P2** (EP-002) |
| Tier | P0 | P0 | P0 (EP-001) |

**Access by user experience** (§146 L3770–3783) is descriptive, not a gating mechanism:

| Feature | Beginner | Experienced | Production team |
|---|---|---|---|
| Idea Vault / Story Board / Screenplay | Yes | Yes | Yes |
| Review | Optional | Yes | Yes |
| Breakdown / Scheduling / Call Sheets | Later | Yes | Yes |
| Shot List / Storyboard | Optional | Yes | Yes |
| AI assistant | Optional | Optional | Optional |

No mechanism for selecting a "beginner" or "pro" mode is specified. The behavior comes from progressive disclosure (§111) and the entry choices in §109.

---

## 4. Personas and workflows that imply concrete behavior

### 4.1 Personas (§3 L192–223; header L10)
- Target users: independent filmmakers, writer-directors, small teams, students, short-film makers, **regional-language filmmakers**, and small episodic teams. What they share is "limited resources", not job title. One person may do several jobs.
- **Implied behavior:** there is one UI for all roles. Roles reveal tools and permissions; there are no separate apps per role (L248). A person can hold several roles at once (e.g., Owner + Writer + Director + Producer, L246).
- **Implied behavior:** "regional-language filmmakers" (L10, L206) and the optional "Language" field (L476) imply that screenplay text, search and PDF export must handle non-Latin scripts. **The PRD states no explicit i18n, font or RTL requirement.** See C-25.

### 4.2 Journeys and the concrete requirements they imply

| Journey | Section | Concrete behaviors implied |
|---|---|---|
| Beginner start choices: "I have an idea" (opens Vault + Story Board), "I already have a screenplay" (opens import/write), "I am ready to plan my shoot" (opens import/breakdown) | §109 L2981–3005 | Three entry routes at project creation. This conflicts with §9.2 (C-10). |
| Pro bypass: Import FDX → Breakdown → Stripboard; New Project → Screenplay → Shot List | §110 L3009–3021 | No tutorial is needed. Every module is reachable without the upstream modules. |
| First Film (17 steps: create → idea → Vault → Acts → Sequences → Cards → drag → Build Screenplay → write → invite reviewer → Draft 2 → lock → breakdown → cast/locations/props → shots/storyboards → stripboard → call sheets) | §162 L4073–4163 | Step 10 "Invite reviewer" must be possible without accounts, which means a package export (C-8). Step 12 "Lock" is P1 (C-11). |
| Existing screenplay: New Project → Import FDX → Review scenes → Breakdown → Confirm → Shots → Schedule → Call Sheets | §163 L4167–4189 | Vault and Story Board can be skipped completely. Import must produce scenes usable by production. |
| Short film | §164 L4193–4207 | Fast path. No forced structure. |
| Writer only (Vault, Story Board, Screenplay, drafts, review) | §165 L4211–4221 | Production must be ignorable. The PRD does not say whether the Production nav should hide. |
| Producer/AD starting from an imported script (Breakdown, Catalog, Cast, Locations, Schedule, Call Sheets) | §166 L4225–4236 | Story tools are available but not required. |
| "A user can ignore half the app": cinematographer uses Shot List + Storyboard; short-film maker uses Screenplay → Breakdown → Call Sheet | §225 L5536–5552 | No module may hard-depend on an upstream module the user skipped. Note that call sheets need a schedule (§82); see C-21. |
| MVP Product Test in one sitting: Create Project → Write Idea → Add Image → Create Act → Create Sequence → **10 Scene Cards** → Drag → Build Screenplay → Write a Scene → Create Draft → Break Down → Create Location → Create Shot → Scene on Shoot Day 1 → Generate Call Sheet → Export PDF | §199 L4866–4904 | This is the canonical P0 end-to-end acceptance path. "If any step feels like switching between unrelated applications, the product is not yet coherent." |
| Unfinished and messy work | §167–168 L4240–4278 | Validation must not demand headings, profiles or addresses. Show "Scene heading optional". Rough text must be accepted on cards. A beat can exist without a scene. |
| Worked examples: Idea→Scene, Script→Breakdown, Script→Shooting Plan, Call Sheet | §211–214 L5077–5268 | §212 shows a user re-categorizing a suggestion ("Rain → move to Sound/Atmosphere note"), so Edit must allow a category change. §213 shows the day summary "3 scenes at same location" plus call and wrap times per day. §214 shows per-actor call times, crew call and scenes with headings. |

### 4.3 UX rules with engineering impact
- One page, one job (§190). Don't show all data everywhere; use the Scene Hub and dedicated views (§189).
- Plain-language labels ("Shooting Schedule", "Scene Cards", "Project Files") (§191 L4715–4729).
- The user experiences only Ideas / Story / Script / Production; Production contains Breakdown, Visuals, Schedule and Call Sheets (§222 L5459–5477). Conflicts with the §7.2 nav (C-9).
- Actions use primary + secondary + "More" menus instead of many buttons. Scene card: Edit, Move, Duplicate, More → Comment, Convert to Scene, Export, Delete (§223 L5481–5511).
- Every feature needs a clear entry point (§226).
- Users do not need to understand data models (§227).

---

## 5. Non-functional requirements

### 5.1 Numeric requirements (all that exist in the PRD)

| NFR | Value | Location |
|---|---|---|
| Supported screenplay size | **120-page** screenplay | §181 L4514 |
| Story cards per project | **150–250** scene cards across development | L4515 |
| Idea Vault items | **"hundreds"** | L4516 |
| Reference images | **"several hundred"** | L4517 |
| Locations | **"dozens"** | L4518 |
| Cast/crew records | **"dozens"** | L4519 |
| Shots | **"hundreds"** | L4520 |
| Side panels open at once | normally **1–2** | §40 L1399 |
| Default breakdown categories | **11** | §60 |
| Comment target kinds | **9** | §50 |
| Project creation | "in **seconds**" | §193 L4761 |
| Act creation | "in **seconds**" | AC §201.1 L4927 |
| Note creation | "in **one interaction**" | AC §200.1 L4912 |
| MVP flow | completable "in **one sitting**", including **10** Scene Cards | §199 L4868, L4881 |
| Short-film minimum | must not require **50** fields | §192 L4735 |

**Not specified anywhere in the PRD:** startup time, UI latency or frame budgets, memory or disk limits, maximum file or attachment size, autosave interval, restore-point frequency, backup retention count, search latency, import/export time, LAN session user limit, package size limits, or AI response time and hardware minimums. §181 explicitly says "This is not a technical architecture requirement, but the UX must remain usable" (L4510). **[FLAG]** The ESD or engineering specs must set these numbers.

### 5.2 Qualitative NFRs (binding behavior)
- **Offline:** every core workflow works offline, including project creation, writing, Story Board, breakdown, scheduling, call sheets and **PDF export** (§5.2 L269–286; AC §209 L5040–5050). AI is never required (L300–307, AC §209.9).
- **Data safety:** no silent data loss (L180). Undo/redo for destructive creative actions (L181). Recovery and backup (L182). Stable behavior when external files or drives become unavailable (L188). No loss when the network drops (§130 L3486). AI failures leave data unchanged (L2754, AC §210.19).
- **Autosave:** the user never has to think about saving. Status shows Saved / Saving… / Unsaved changes (§105).
- **Deletion:** soft delete through Recently Deleted. Permanent delete is a deliberate second action (§101).
- **Import safety:** never auto-replace. Default to a new draft (§44). Package import never silently overwrites (§90, AC §208.5).
- **Privacy:** private notes are never exported or shared by default (§51, §124 L3354). Moodboard internal notes are excluded unless selected (§68).
- **Portability and ownership:** Export Project Archive; the user chooses storage location; no OpenFrame cloud folder (§106–107). "The user owns and can move the project" (L2946).
- **Responsiveness:** "Application responsiveness for normal supported project sizes" (L187). Sizes are given in §181.
- **Accessibility:** only "Accessibility/basic input flexibility" (L186) and keyboard-first operation (§103). There is no WCAG level or screen-reader requirement. **[FLAG]**
- **Print and export fidelity:** readable on standard paper sizes (§125). Screenplay export preserves page breaks, scene numbers, title page and revisions (§45). Call-sheet PDFs are readable on a phone (§85).
- **Windowing:** resizable and fullscreen; core work fits one window (§184).
- **Determinism:** AI counts and statistics must come from canonical data and deterministic calculation (L2703, AC §210.5). Changing the model must not alter semantics or authorization (AC §210.20).

---

## 6. Cloud AI, external AI, accounts, telemetry and sync

**Approved build decision:** LOCAL AI ONLY, no accounts, no telemetry by default. The Engineering Package Index (L30–51) records "AI providers: Local AI only for v1; no cloud AI provider integration", "Accounts: No account required", and "Telemetry: Off by default; optional anonymous diagnostics/crash reports with explicit consent".

### 6.1 PRD passages that conflict with, or need resolution under, LOCAL AI ONLY

| ID | Line(s) | PRD text (abridged) | Conflict / resolution needed |
|---|---|---|---|
| A-1 | L185 | Cross-cutting: "Clear indication when external AI services are used." | Under local-only, external AI is never used. Proposed resolution: keep this as a guard requirement (the UI must still show "local, on-device" status), but no external path ships in v1. |
| A-2 | **L296** | "the product may connect to a **configured AI provider** or a locally available model." | **Direct conflict.** It permits cloud providers. Resolution: v1 = local model only; the provider abstraction may exist internally but no external provider is exposed. |
| A-3 | L298 | "The interface must clearly indicate when content is being sent outside the local application." | Vacuous for AI in v1, but it still applies to any outbound network use (model download, URL fetch, update check). Keep it as a general egress-disclosure rule. |
| A-4 | L2606 | "an **Amazon Q-style** assistant" | An analogy to a cloud product. Interaction style only; it does not imply cloud. |
| A-5 | **L2752** | "**External AI providers remain optional** and require clear disclosure before project content is transmitted." | **Direct conflict.** It presumes external providers exist as an option. Resolution: document them as out of scope for v1. |
| A-6 | L2746 | "A small local model, including a **Qwen2.5-0.5B-class** model, is a supported implementation target" | **Consistent.** The Engineering Index adds hardware-aware automatic Qwen profile selection and "One-click Download Offline AI" from an OpenFrame-controlled CDN. Note that the model download is outbound network use (see A-3). |
| A-7 | **L5072** | AC §210.17: "External AI usage is clearly disclosed before project content is transmitted externally." | Acceptance criterion for a v1-absent path. Resolution: mark N/A for v1, or test that no external transmission path exists. |
| A-8 | **L5073** | AC §210.18: "Local AI **may** operate without internet when configured and supported." | Under local-only this should be strengthened to **must** once the model is installed. |
| A-9 | L5075 | AC §210.20: changing the underlying model must not alter semantics | Consistent. It applies to swapping Qwen profiles. |
| A-10 | L671 | Voice note: "If **speech-to-text** is available, transcription is an optional convenience" | STT is AI/ML. Under local-only it must be an on-device model or be omitted. It is untiered. **[FLAG]** |
| A-11 | L649–651 | URL item stores "title if available" and an "optional screenshot/thumbnail" | Implies outbound HTTP fetches of third-party pages. Not AI or telemetry, but it is network egress. It should be user-initiated and fail gracefully offline. **[FLAG]** |
| A-12 | L164 | Good automation: "Generate a synopsis from text when the user asks." | Requires the AI (P1) or it is a deterministic stub. Local model only. |
| A-13 | L2056 | Storyboard: "should not automatically invent a complete visual sequence unless the user asks an AI assistant to help." | Implies AI-generated storyboard content. A 0.5B-class local **text** model cannot generate images. Resolution: text-only panel descriptions, or out of scope. **[FLAG]** |
| A-14 | L2214–2220 | P0 "Schedule Assistance": the user "asks: 'Suggest a more practical shooting order.' The **assistant** may point out…" | The **tier conflicts**: this is P0 (SCHED-001), but the AI Assistant is P1 (AI-001). Resolution: P0 must ship deterministic grouping suggestions (same location/cast) without AI. |
| A-15 | L1804, L4859 | P0 "Automatic Breakdown Suggestions" ("The system may analyze the scene") | Must work with no AI (§5.3 L300–307: AI is never required to break down a scene). Resolution: deterministic or heuristic extraction (character cues, headings, capitalized props) in P0. The AI version is additive (§94 lists "suggest breakdown elements"). |
| A-16 | L5530–5531 | Filmmaker ownership affects "AI privacy, cloud requirements" | Consistent. |
| A-17 | §146 L3783 | AI "Optional" for all users | Consistent. |

### 6.2 Accounts and identity

The PRD never requires an account. However, several features assume **user identity**, which must be supplied without accounts (for example, a local display-name profile). Each needs a resolution:

| ID | Line(s) | Passage | Issue |
|---|---|---|---|
| B-1 | L366 | Shell top bar: "Help / **User**" | Implies a user menu. It must be a local profile, not an account. |
| B-2 | L2441 | "No OpenFrame **cloud account** is required for exchanging a package." | Worded narrowly, as if accounts might exist elsewhere. Under the build decision, no accounts exist anywhere. |
| B-3 | L229–248, L3443–3467 | Roles (8) and Permissions (Owner/Editor/Commenter/Viewer/Export-only) | Permissions need principal identities. Without accounts, it is unclear how Commenter/Viewer is enforced on a received package or LAN client. **[FLAG]** |
| B-4 | L2748, AC §210.14 | "AI inherits the user's effective permissions" | Needs a local notion of the current user. |
| B-5 | L3568 | Review package shows "**Exported by**" | Needs a local display name. |
| B-6 | L3722 | Task "optional **assignee**" | Needs a people list (Cast/Crew records or local profiles). |
| B-7 | L3517–3519 | LAN locks show "the editor currently working there" | Needs a session identity for LAN peers. |
| B-8 | L4133 | Journey step 10: "**Invite reviewer**" | "Invite" suggests account invitations. Resolution: Export Review Package. |
| B-9 | L1600, L1666 | Review round "reviewer(s)"; lock "undo for **authorized users**" | Identity and authorization without accounts. |

### 6.3 Telemetry
- **The PRD contains no mention of telemetry, analytics, crash reporting or usage tracking.** There is nothing to conflict with "no telemetry by default."
- Activity History (§144) is local project orientation, "not enterprise auditing". It is not telemetry.
- Cross-spec: the Engineering Index (L43) and ESD (L43, L312, L346) allow **opt-in** anonymous diagnostics and crash reports. This is consistent with "off by default", but opt-in crash reporting is a capability the PRD never mentions (the PRD is silent rather than conflicting).
- **Updates:** the PRD is silent. The Engineering Index says "Notify user; download/install only after explicit approval." An update check is network egress (see A-3).

### 6.4 Sync / cloud
| ID | Line(s) | Passage | Status |
|---|---|---|---|
| S-1 | L2395 | "The user does not want to maintain a cloud synchronization platform" | Consistent. |
| S-2 | L2437, L2598, L3555 | Users may use email, WhatsApp, Telegram, Drive or Dropbox themselves as transport | Consistent. There is no integration with those services, and OpenFrame doesn't own the channel. |
| S-3 | L2959 | Project on a "network drive where supported by the OS" | Consistent with no-cloud. **[ENG NOTE]** SQLite on network or cloud-synced folders is a known corruption and locking risk, so this needs an explicit ESD position. |
| S-4 | L2445–2457, §131 | LAN collaboration (direct peer-to-host, no cloud) | Consistent with no-cloud. It requires a network listener, discovery and identity. Tier conflict with the Engineering Index (C-18). |
| S-5 | L5377 | DaVinci Resolve "cloud collaboration" cited as research | Context only. |
| S-6 | L4862 | Deferred: "cloud media hosting" | Consistent. |
| S-7 | L5706 | "Not a cloud collaboration company." | Consistent. |

---

## 7. Contradictions, gaps and counter-intuitive rules

### 7.1 Internal contradictions and ambiguities

| # | Lines | Issue |
|---|---|---|
| C-1 | L971–972, L994 vs L1167, L4254–4258, L4931, L3034 | **Scene heading rules.** §21.1 says the heading lives only in the detailed view and the minimized card shows only the description (L971). §21.2 says the heading is "optional … but **mandatory before converting to screenplay**" (L994). But §28 says "the **optional** scene heading is carried across" (L1167). §167 forbids "You must add a scene heading" (L4254). AC §201.5 says "Scene heading is optional." And the §111 example shows a compact card displaying "INT. HOUSE — NIGHT" (L3034). Decide whether Build Screenplay blocks on missing headings, and what the compact card shows. |
| C-2 | L1188–1202 vs L1150, L1172, L3108, L5442 | **Story Board ↔ Screenplay coupling.** §29 says that reordering cards after the screenplay has written scenes offers "Apply" to **change screenplay scene order**. That requires a persistent card↔scene link and a reverse propagation path. This contradicts "no requirement for constant synchronization" (L1150), "Scene Card remains a separate reference object" (L1172), "Story Board does not automatically rewrite screenplay content" (L3108) and Rule #6 (L5442). The §114 source of truth for story order ("Active Story Board **or** screenplay after written", L3125) is ambiguous. |
| C-3 | L4855, L4866; registry L4806, L4815–4841 | **Missing sections 196–198.** The heading "195. Feature Priority — P0" contains the P0, P1, P2 and Deferred tiers, and numbering jumps from 195 to 199. The registry still cites §196 (P1), §197 (P2) and §197–198 for 12 IDs (SCRIPT-007, EP-002, STB-002/003, SCHED-002/003, COL-004, AI-002, PROD-007, CORE-004). The FSD (L4545–4547) still maps §196 "P1", §197 "P2" and §198 "Explicitly Deferred". Those features have **no behavioral description** in this PRD beyond one-line tier entries. |
| C-4 | Registry vs §195 | **Tier sources disagree on coverage.** In §195 but not the registry: Scene Hub (P1), more export layouts (P1), multi-monitor (P1), richer exchange controls (≈COL-004). In the body but in neither source: §88, 90, 114–119, 128, 135, 147–153, 170, 172–175, 181, 184, 186. Production handoff and reconciliation (§147–153) are called a "critical control point" (L3846) yet have no ID. |
| C-5 | L4834, L4860 vs L4862, L1006, L13 | **Lightweight Budget Snapshot (P1, PRD-PROD-005)** has no PRD section. It cites "140, 154 and budgeting references", but §140 is Research boards and §154 is Reports. Meanwhile "full budgeting" is Deferred and "budget" is listed as not a Scene Card field. The Domain spec (L846, L1471) and FSD (L1468) do define it, so this is a capability that appears in other specs without a PRD body, which violates the rule at L47. |
| C-6 | L1375 vs L1636–1646 | **"Private notes" has two meanings.** §39 calls screenplay scene notes "private notes outside the printed screenplay" (non-printing). §51 "Private notes" are not shared with collaborators. Decide whether §39 scene notes travel in review packages or LAN sessions. |
| C-7 | L231–240 vs L3447–3465 | **Roles vs permissions mismatch.** §4 lists 8 roles (Owner, Writer, Director, Producer, AD, Reviewer, Contributor, Viewer). §129 lists 5 permission levels (Owner, Editor, Commenter, Viewer, Export-only). No mapping is given. |
| C-8 | L4133 | "Invite reviewer" in a no-account, no-cloud product means Export Review Package. See B-8. |
| C-9 | L389–398 vs L5459–5477, L5570–5576 | **Navigation model conflict.** §7.2 has 8 top-level items, with Breakdown and Call Sheets at top level. §222 says the user experiences only Ideas / Story / Script / Production, with Breakdown, Visuals, Schedule and Call Sheets **inside** Production. §226 mixes both ("Production requirements → Breakdown"; "Scheduling → Production → Schedule"; "Call sheet → Call Sheets"). The §6 information model (L340–349) puts Breakdown and Call Sheets under Production. |
| C-10 | L463–501 vs L2985–3005, L4757–4763 | **Project-creation flow conflict.** §9 asks for title + type and then lands on Project Home with 4 buttons. §109 says "At project creation, offer three clear options" (idea / screenplay / plan shoot). §193 says there is no setup wizard. Decide whether the three options appear in the New Project dialog or on the Home landing. |
| C-11 | L4141, L5294, L73–74 vs L4803 | **Lock is drawn as a gate but tiered P1.** The thesis flow (L72–74), First Film step 12, and the §215 data flow ("LOCKED SHOOTING DRAFT" feeding Breakdown) show locking before breakdown. But Script Lock is **P1** and §8 allows breakdown before lock (L453). §118 L3214 refers to a "locked scene". The P0 production handoff (§147) therefore must work from any draft. |
| C-12 | L3787–3904 | **Production source/handoff is untiered** yet P0 ACs depend on it (§203.8–9, §207.6). It also conflicts with §112 "Screenplay → Breakdown: direct production extraction" if the source draft is implicit. |
| C-13 | L4860 vs L4696, L4839 | **Scene Hub is P1** (§195), but §189's UX rule ("provide a clear Scene Hub", part of UX-001 P0) depends on it, and §171 calls it "a high-value connection feature". |
| C-14 | L1239 vs L4798 | **Character Relationship View** is in PRD-STORY-006 (P0) but the body says "The relationship map is optional." It is unclear whether "optional" means optional for the product or for the user. |
| C-15 | L3732 vs L4835 | **Tasks:** "That is enough for **MVP**" (§143) vs Tasks tiered **P1** (PROD-006). §229 L5651 also lists Tasks as a "Supporting surface". |
| C-16 | L4822 vs L4823, L4826 | **Permissions (COL-001) are P1**, but P0 Review Packages (COL-002) and P0 LAN collaboration (COL-005) need at least Commenter/Viewer semantics (the reviewer "can read, annotate, comment", L2502–2507; the host picks "allowed pages", L3499). |
| C-17 | L4826–4827 | **COL-005 (P0) and COL-006 (P1) both cite §131–132.** The boundary between "basic" soft-locking and "advanced conflict handling" is undefined. |
| C-18 | L4826, L4859 vs Engineering Index | **LAN collaboration tier.** The PRD makes basic LAN collaboration **P0**. The Engineering Package Index's locked decision is "LAN collaboration: Architecture-ready now; **ship after stable solo core**" and "Soft locks + optimistic version/conflict handling; no CRDT in v1". This effectively re-tiers a P0 feature, which violates the PRD rule that only the PRD sets priority (L47, L4849). The PRD needs an amendment. |
| C-19 | L8, L256–259 vs Engineering Index | **OS scope.** The PRD requires Windows **and** macOS as P0 (L4859 "Desktop Windows/macOS"). The Engineering Index says "Windows 11 first; preserve macOS-ready boundaries." This also needs a PRD amendment or a sequencing note. |
| C-20 | L2550–2574 vs L3598–3646 | **Exchange package set.** Six extensions are named, but behavior is described only for Script review (§89), Story (§136), Shots (§137) and Breakdown (§138). `.ofschedule` and `.ofcallreview` have no described workflow. §91 says "Every major OpenFrame workspace can produce" a package, but COL-003 is P1. |
| C-21 | L5548, L4192 vs L2306 | "Screenplay → Breakdown → Call Sheet" (§225) skips the schedule, but call sheets are "generated from a scheduled shooting day" (§82) and must "derive from shooting days" (Rule #10). Either a minimal implicit shooting day is needed, or the §225 phrase is loose. |
| C-22 | L1735–1736 | **Episodic production is per-episode.** Each episode has its own breakdown and production plan, so no cross-episode ("block") scheduling is described. This contradicts the common episodic practice of shooting several episodes together. The series-level shared reference covers only characters and locations. |
| C-23 | L2316–2358 | **Call sheet has no crew roster or department call times.** It has only "Crew call", cast call times and emergency contacts. This contradicts the common call-sheet assumption. The Crew directory (§66) does not feed the call sheet per §112. |
| C-24 | L3370, L2138 | **Unspecified industry formats.** Paper size is only "standard paper sizes" (US Letter vs A4; both matter for regional users). Strip "page count" does not say eighths, the industry convention. |
| C-25 | L10, L206, L476 | **Regional-language users** are targeted, but there is no requirement for non-Latin script input, fonts, PDF embedding, RTL, or FDX/Fountain round-trip of Unicode. **[FLAG]** |
| C-26 | L733–739 | "Send to Story Board" targets "Sequence idea", "Character note" and "Story note". These object types are not defined anywhere else in the Story model (§16–32). It is unclear whether "Character note" creates a Character record. |
| C-27 | L431–449 vs L3269–3279 vs L3981 | **Three lifecycle/status vocabularies.** The §8 lifecycle has 9 stages (e.g., "Shooting Draft", "Shooting Preparation", "Call Sheets", "Shoot"). The §121 status has 9 values ("Locked", "Completed", "Archived"). §157 uses "Shooting Complete". They are not aligned. |
| C-28 | L1463, L1475, L3797 vs L334–338 | **Multiple screenplays per project?** Import offers "Import as New Screenplay", §44 mentions "replace or import into another project", and §147 asks "which screenplay/draft". But the §6 model shows a single Screenplay per project (except per episode). Cardinality is unspecified. |
| C-29 | L1078, L870 vs L5282–5287 | Beat placement: beats live under Sequences (L870, L1078), but the §215 diagram shows Act, Sequence and Beat as siblings, with Scene Card under Sequence. It is unclear whether Beats can sit directly under Acts or in the Parking Lot. |
| C-30 | L2373 vs L3114 | Call sheet edits may update the schedule "if the user explicitly chooses", so a **call sheet → schedule write-back path** is implied. §112 only defines Schedule → Call Sheet. |
| C-31 | L1166–1168 | "Build Screenplay" when a screenplay already exists: behavior is undefined (append, new draft, or refuse?). §44's import-safety rule covers only imports. |
| C-32 | L1438, 1770, 2060, 2143, 2457, 3527, 5359, 5365, 5371, 5377 | **Citation artifacts** ("citeturn…search…") are left in the PRD text. These are editorial debris and should be stripped. |
| C-33 | L4799, L5624; L4833, L5644 | §229's "Core" surfaces include P1 items (Story Timeline, Sides, Tasks), so "core" ≠ P0. Tiers must come from the registry, not from §229. |
| C-34 | L4806 | SCRIPT-007 cites "§197–198", but §198 was the Deferred list in the older version (per FSD L4547). The mapping is likely stale. |

### 7.2 Rules that contradict common assumptions (intentional; engineers should not "fix" them)

| # | Lines | Counter-intuitive rule |
|---|---|---|
| N-1 | L1027–1035, L3143–3145, L3161 | **Scene numbers are always derived from screenplay order and renumber when scenes move**, even after production starts (§116 says "will update its script number"). This contradicts the industry convention of **frozen scene numbers after lock** (inserted scenes numbered 12A, removed scenes marked "OMITTED"). §54 revision pages don't address it. Shot numbers such as "24A" (L2073) are derived from scene numbers, so it is unclear whether shots renumber too. **[FLAG]** A decision is needed for post-lock numbering. |
| N-2 | L1674 | Locking does **not** make a draft immutable. It adds a "Create a revision?" prompt. Lock is undoable by authorized users (L1656). |
| N-3 | L570–572, L741–751, L4469–4471 | Vault → Project and Vault → Story are **copies**, never live links. Only a "Used in Story Board" breadcrumb remains. |
| N-4 | L1150, L3108–3110 | Story Board and Screenplay are **not synchronized**. Editing dialogue never updates cards. (Caveat: C-2.) |
| N-5 | L3155–3168, L3898–3904 | **Story order ≠ shooting order.** Script changes never reorder the schedule. |
| N-6 | L1830, L5444 | Breakdown suggestions **never** auto-apply, and script changes **never** auto-remove breakdown items (§117). |
| N-7 | L2373, L3114 | Call sheet edits don't flow back to the schedule automatically. Call sheets become stale ("Based on an older schedule version") rather than auto-updating. |
| N-8 | L3815–3823 | Production is pinned to a **Production Source draft**. New drafts don't affect production until an explicit "Update Production from Draft N". |
| N-9 | L1085–1089, L3432–3439 | Drag/drop and reorder have **no confirmation dialogs**. Undo is the safety net. Only the destructive actions listed in §128 confirm. |
| N-10 | L1015–1025 | Story Board cards have **no numbers**. Letters (A/B/C) are shown in the outline. |
| N-11 | L840, L2024 | Terminology: **"Story Board"** (two words) is the outline card wall, while **"Storyboard"** (one word) is visual shot panels. These must be distinct entities and labels in code. |
| N-12 | L2175–2183 | The "Boneyard" holds **all** scenes initially. Scheduling is purely manual drag-and-drop, with no auto-scheduler. |
| N-13 | L1357 | In-script search is deliberately **non-semantic**. |
| N-14 | L1407–1421 | The Idea Vault must **not** be embeddable as a live writing sidebar, although the Story Board, Characters and Comments can be. |
| N-15 | L2389 | Mobile support is **only** through exported PDFs. |
| N-16 | L2746 | The AI model is **not trusted** for counts, permissions, validation or writes. All of those are deterministic application logic. |

### 7.3 Cross-spec conflicts found during this read (not exhaustive)
- **Engineering Package Index vs PRD:** LAN collaboration timing (C-18); Windows-first vs Windows+macOS P0 (C-19); local-only AI vs PRD provider language (A-2, A-5, A-7). The opt-in crash telemetry and update-notify behavior in the Index/ESD are **not described in the PRD**.
- **FSD vs PRD:** the FSD traceability table (L4545–4547) references PRD §196–198, which no longer exist in this PRD (C-3). The FSD and Domain spec define Budget Snapshot behavior with no PRD body section (C-5).
- **Filename drift:** the Engineering Index lists `OpenFrame_Studio_Mega_PRD_Aligned_Updated(3).md`, but the file on disk has no "(3)" suffix. Confirm they are the same revision.

---
